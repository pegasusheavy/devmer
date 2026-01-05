import { injectable, inject } from 'tsyringe';
import type {
  IContactService,
  IEmailService,
  ILeadService,
  ContactSubmission,
  ContactSubmissionData,
  ContactResponseData,
  ContactFilter,
  PaginatedResult,
} from '../types';
import { TOKENS } from '../container';
import type { PrismaService } from './prisma.service';

@injectable()
export class ContactService implements IContactService {
  constructor(
    @inject(TOKENS.Prisma) private prisma: PrismaService,
    @inject(TOKENS.Email) private emailService: IEmailService,
    @inject(TOKENS.Lead) private leadService: ILeadService
  ) {}

  async submit(data: ContactSubmissionData): Promise<ContactSubmission> {
    const client = this.prisma.client;

    const submission = await client.contactSubmission.create({
      data: {
        email: data.email,
        name: data.name,
        company: data.company,
        subject: data.subject,
        message: data.message,
        type: (data.type as never) || 'GENERAL',
        status: 'NEW',
        ipAddress: data.ipAddress,
        userAgent: data.userAgent,
        referrer: data.referrer,
      },
    });

    // Send acknowledgment email
    await this.emailService.sendTemplate('contact-received', data.email, {
      name: data.name,
      message: data.message,
    });

    // Create or update lead
    const existingLead = await this.leadService.findByEmail(data.email);
    if (existingLead) {
      await this.leadService.recordActivity(existingLead.id, {
        type: 'FORM_SUBMIT',
        description: `Contact form submission: ${data.subject || data.type || 'General inquiry'}`,
        metadata: { submissionId: submission.id },
      });
    } else {
      await this.leadService.create({
        email: data.email,
        firstName: data.name.split(' ')[0],
        lastName: data.name.split(' ').slice(1).join(' ') || undefined,
        company: data.company,
        source: 'contact_form',
      });
    }

    return this.mapSubmission(submission);
  }

  async findById(id: string): Promise<ContactSubmission | null> {
    const client = this.prisma.client;
    const submission = await client.contactSubmission.findUnique({ where: { id } });
    return submission ? this.mapSubmission(submission) : null;
  }

  async respond(id: string, response: ContactResponseData): Promise<ContactSubmission> {
    const client = this.prisma.client;

    const submission = await client.contactSubmission.update({
      where: { id },
      data: {
        status: 'RESPONDED',
        respondedAt: new Date(),
        respondedBy: response.respondedBy,
        notes: response.notes,
      },
    });

    return this.mapSubmission(submission);
  }

  async list(filter: ContactFilter): Promise<PaginatedResult<ContactSubmission>> {
    const client = this.prisma.client;
    const limit = filter.limit || 20;
    const offset = filter.offset || 0;

    const where: Record<string, unknown> = {};

    if (filter.status) {
      where['status'] = filter.status;
    }

    if (filter.type) {
      where['type'] = filter.type;
    }

    if (filter.search) {
      where['OR'] = [
        { email: { contains: filter.search, mode: 'insensitive' } },
        { name: { contains: filter.search, mode: 'insensitive' } },
        { company: { contains: filter.search, mode: 'insensitive' } },
        { message: { contains: filter.search, mode: 'insensitive' } },
      ];
    }

    const [submissions, total] = await Promise.all([
      client.contactSubmission.findMany({
        where,
        take: limit,
        skip: offset,
        orderBy: { createdAt: 'desc' },
      }),
      client.contactSubmission.count({ where }),
    ]);

    return {
      data: submissions.map(this.mapSubmission),
      total,
      limit,
      offset,
      hasMore: offset + submissions.length < total,
    };
  }

  private mapSubmission(sub: Record<string, unknown>): ContactSubmission {
    return {
      id: sub['id'] as string,
      email: sub['email'] as string,
      name: sub['name'] as string,
      company: sub['company'] as string | undefined,
      subject: sub['subject'] as string | undefined,
      message: sub['message'] as string,
      type: sub['type'] as string,
      status: sub['status'] as string,
      createdAt: sub['createdAt'] as Date,
    };
  }
}
