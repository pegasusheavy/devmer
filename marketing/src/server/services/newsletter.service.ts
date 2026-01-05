import { injectable, inject } from 'tsyringe';
import type {
  INewsletterService,
  IEmailService,
  IConfigService,
  Subscriber,
  NewsletterPreferences,
  SubscriberFilter,
  PaginatedResult,
} from '../types';
import { TOKENS } from '../container';
import type { PrismaService } from './prisma.service';

@injectable()
export class NewsletterService implements INewsletterService {
  constructor(
    @inject(TOKENS.Prisma) private prisma: PrismaService,
    @inject(TOKENS.Email) private emailService: IEmailService,
    @inject(TOKENS.Config) private configService: IConfigService
  ) {}

  async subscribe(email: string, preferences?: NewsletterPreferences): Promise<Subscriber> {
    const client = this.prisma.client;

    // Check if already exists
    const existing = await client.newsletterSubscriber.findUnique({
      where: { email },
    });

    if (existing) {
      // Reactivate if unsubscribed
      if (existing.status === 'UNSUBSCRIBED') {
        const updated = await client.newsletterSubscriber.update({
          where: { email },
          data: {
            status: 'PENDING',
            preferences: preferences as object,
            unsubscribedAt: null,
          },
        });
        return this.mapSubscriber(updated);
      }
      return this.mapSubscriber(existing);
    }

    const subscriber = await client.newsletterSubscriber.create({
      data: {
        email,
        status: 'PENDING',
        preferences: preferences as object,
      },
    });

    // Send confirmation email
    const appUrl = this.configService.get<string>('app.url');
    const confirmToken = this.generateToken();

    await this.emailService.sendTemplate('newsletter-confirm', email, {
      confirmUrl: `${appUrl}/api/newsletter/confirm?email=${encodeURIComponent(email)}&token=${confirmToken}`,
    });

    return this.mapSubscriber(subscriber);
  }

  async unsubscribe(email: string): Promise<void> {
    const client = this.prisma.client;

    await client.newsletterSubscriber.update({
      where: { email },
      data: {
        status: 'UNSUBSCRIBED',
        unsubscribedAt: new Date(),
      },
    });
  }

  async confirm(email: string, _token: string): Promise<void> {
    const client = this.prisma.client;

    // In a real implementation, verify the token
    await client.newsletterSubscriber.update({
      where: { email },
      data: {
        status: 'ACTIVE',
        confirmedAt: new Date(),
      },
    });
  }

  async updatePreferences(email: string, preferences: NewsletterPreferences): Promise<Subscriber> {
    const client = this.prisma.client;

    const subscriber = await client.newsletterSubscriber.update({
      where: { email },
      data: {
        preferences: preferences as object,
      },
    });

    return this.mapSubscriber(subscriber);
  }

  async getSubscribers(filter: SubscriberFilter): Promise<PaginatedResult<Subscriber>> {
    const client = this.prisma.client;
    const limit = filter.limit || 20;
    const offset = filter.offset || 0;

    const where: Record<string, unknown> = {};

    if (filter.status) {
      where['status'] = filter.status;
    }

    const [subscribers, total] = await Promise.all([
      client.newsletterSubscriber.findMany({
        where,
        take: limit,
        skip: offset,
        orderBy: { createdAt: 'desc' },
      }),
      client.newsletterSubscriber.count({ where }),
    ]);

    return {
      data: subscribers.map(this.mapSubscriber),
      total,
      limit,
      offset,
      hasMore: offset + subscribers.length < total,
    };
  }

  private generateToken(): string {
    return Math.random().toString(36).substring(2, 15) +
           Math.random().toString(36).substring(2, 15);
  }

  private mapSubscriber(sub: Record<string, unknown>): Subscriber {
    return {
      id: sub['id'] as string,
      email: sub['email'] as string,
      firstName: sub['firstName'] as string | undefined,
      status: sub['status'] as string,
      preferences: sub['preferences'] as NewsletterPreferences | undefined,
      createdAt: sub['createdAt'] as Date,
    };
  }
}
