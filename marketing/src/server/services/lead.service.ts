import { injectable, inject } from 'tsyringe';
import type {
  ILeadService,
  IEmailService,
  Lead,
  CreateLeadData,
  UpdateLeadData,
  LeadActivityData,
  LeadFilter,
  PaginatedResult,
} from '../types';
import { TOKENS } from '../container';
import type { PrismaService } from './prisma.service';

@injectable()
export class LeadService implements ILeadService {
  constructor(
    @inject(TOKENS.Prisma) private prisma: PrismaService,
    @inject(TOKENS.Email) private emailService: IEmailService
  ) {}

  async create(data: CreateLeadData): Promise<Lead> {
    const client = this.prisma.client;

    const lead = await client.lead.create({
      data: {
        email: data.email,
        firstName: data.firstName,
        lastName: data.lastName,
        company: data.company,
        jobTitle: data.jobTitle,
        source: data.source,
        campaign: data.campaign,
        score: 10, // Initial score for new leads
      },
    });

    // Record creation activity
    await this.recordActivity(lead.id, {
      type: 'LEAD_CREATED',
      description: `Lead created from ${data.source || 'unknown source'}`,
    });

    // Send welcome email
    await this.emailService.sendTemplate('welcome', lead.email, {
      name: data.firstName || 'there',
    });

    return this.mapLead(lead);
  }

  async findById(id: string): Promise<Lead | null> {
    const client = this.prisma.client;
    const lead = await client.lead.findUnique({ where: { id } });
    return lead ? this.mapLead(lead) : null;
  }

  async findByEmail(email: string): Promise<Lead | null> {
    const client = this.prisma.client;
    const lead = await client.lead.findUnique({ where: { email } });
    return lead ? this.mapLead(lead) : null;
  }

  async update(id: string, data: UpdateLeadData): Promise<Lead> {
    const client = this.prisma.client;

    const lead = await client.lead.update({
      where: { id },
      data: {
        firstName: data.firstName,
        lastName: data.lastName,
        company: data.company,
        jobTitle: data.jobTitle,
        status: data.status as never,
        score: data.score,
      },
    });

    return this.mapLead(lead);
  }

  async updateScore(id: string, scoreChange: number): Promise<Lead> {
    const client = this.prisma.client;

    const lead = await client.lead.update({
      where: { id },
      data: {
        score: { increment: scoreChange },
      },
    });

    return this.mapLead(lead);
  }

  async recordActivity(leadId: string, activity: LeadActivityData): Promise<void> {
    const client = this.prisma.client;

    await client.leadActivity.create({
      data: {
        leadId,
        type: activity.type as never,
        description: activity.description,
        metadata: activity.metadata as object,
      },
    });

    // Update lead score based on activity type
    const scoreMap: Record<string, number> = {
      PAGE_VIEW: 1,
      DOWNLOAD: 5,
      FORM_SUBMIT: 10,
      EMAIL_OPEN: 2,
      EMAIL_CLICK: 3,
      DEMO_REQUEST: 20,
      PRICING_VIEW: 5,
      DOCS_VIEW: 3,
      GITHUB_STAR: 5,
    };

    const scoreChange = scoreMap[activity.type] || 0;
    if (scoreChange > 0) {
      await this.updateScore(leadId, scoreChange);
    }
  }

  async list(filter: LeadFilter): Promise<PaginatedResult<Lead>> {
    const client = this.prisma.client;
    const limit = filter.limit || 20;
    const offset = filter.offset || 0;

    const where: Record<string, unknown> = {};

    if (filter.status) {
      where['status'] = filter.status;
    }

    if (filter.source) {
      where['source'] = filter.source;
    }

    if (filter.minScore !== undefined) {
      where['score'] = { gte: filter.minScore };
    }

    if (filter.search) {
      where['OR'] = [
        { email: { contains: filter.search, mode: 'insensitive' } },
        { firstName: { contains: filter.search, mode: 'insensitive' } },
        { lastName: { contains: filter.search, mode: 'insensitive' } },
        { company: { contains: filter.search, mode: 'insensitive' } },
      ];
    }

    const [leads, total] = await Promise.all([
      client.lead.findMany({
        where,
        take: limit,
        skip: offset,
        orderBy: { createdAt: 'desc' },
      }),
      client.lead.count({ where }),
    ]);

    return {
      data: leads.map(this.mapLead),
      total,
      limit,
      offset,
      hasMore: offset + leads.length < total,
    };
  }

  private mapLead(lead: Record<string, unknown>): Lead {
    return {
      id: lead['id'] as string,
      email: lead['email'] as string,
      firstName: lead['firstName'] as string | undefined,
      lastName: lead['lastName'] as string | undefined,
      company: lead['company'] as string | undefined,
      jobTitle: lead['jobTitle'] as string | undefined,
      score: lead['score'] as number,
      status: lead['status'] as string,
      source: lead['source'] as string | undefined,
      createdAt: lead['createdAt'] as Date,
      updatedAt: lead['updatedAt'] as Date,
    };
  }
}
