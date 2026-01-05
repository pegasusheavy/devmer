import { injectable, inject } from 'tsyringe';
import type {
  IAnalyticsService,
  PageViewData,
  EventData,
  AnalyticsFilter,
} from '../types';
import { TOKENS } from '../container';
import type { PrismaService } from './prisma.service';

@injectable()
export class AnalyticsService implements IAnalyticsService {
  constructor(
    @inject(TOKENS.Prisma) private prisma: PrismaService
  ) {}

  async trackPageView(data: PageViewData): Promise<void> {
    const client = this.prisma.client;

    await client.pageView.create({
      data: {
        path: data.path,
        referrer: data.referrer,
        userAgent: data.userAgent,
        ipAddress: data.ipAddress,
        sessionId: data.sessionId,
        visitorId: data.visitorId,
        utmSource: data.utmSource,
        utmMedium: data.utmMedium,
        utmCampaign: data.utmCampaign,
      },
    });
  }

  async trackEvent(data: EventData): Promise<void> {
    const client = this.prisma.client;

    await client.event.create({
      data: {
        name: data.name,
        category: data.category,
        properties: data.properties as object,
        sessionId: data.sessionId,
        visitorId: data.visitorId,
        path: data.path,
      },
    });
  }

  async getPageViews(filter: AnalyticsFilter): Promise<PageViewData[]> {
    const client = this.prisma.client;

    const where: Record<string, unknown> = {};

    if (filter.startDate || filter.endDate) {
      where['createdAt'] = {};
      if (filter.startDate) {
        (where['createdAt'] as Record<string, unknown>)['gte'] = filter.startDate;
      }
      if (filter.endDate) {
        (where['createdAt'] as Record<string, unknown>)['lte'] = filter.endDate;
      }
    }

    if (filter.path) {
      where['path'] = filter.path;
    }

    const pageViews = await client.pageView.findMany({
      where,
      take: filter.limit || 100,
      skip: filter.offset || 0,
      orderBy: { createdAt: 'desc' },
    });

    return pageViews.map((pv) => ({
      path: pv.path,
      referrer: pv.referrer || undefined,
      userAgent: pv.userAgent || undefined,
      ipAddress: pv.ipAddress || undefined,
      sessionId: pv.sessionId || undefined,
      visitorId: pv.visitorId || undefined,
      utmSource: pv.utmSource || undefined,
      utmMedium: pv.utmMedium || undefined,
      utmCampaign: pv.utmCampaign || undefined,
    }));
  }

  async getEvents(filter: AnalyticsFilter): Promise<EventData[]> {
    const client = this.prisma.client;

    const where: Record<string, unknown> = {};

    if (filter.startDate || filter.endDate) {
      where['createdAt'] = {};
      if (filter.startDate) {
        (where['createdAt'] as Record<string, unknown>)['gte'] = filter.startDate;
      }
      if (filter.endDate) {
        (where['createdAt'] as Record<string, unknown>)['lte'] = filter.endDate;
      }
    }

    const events = await client.event.findMany({
      where,
      take: filter.limit || 100,
      skip: filter.offset || 0,
      orderBy: { createdAt: 'desc' },
    });

    return events.map((e) => ({
      name: e.name,
      category: e.category || undefined,
      properties: e.properties as Record<string, unknown> | undefined,
      sessionId: e.sessionId || undefined,
      visitorId: e.visitorId || undefined,
      path: e.path || undefined,
    }));
  }
}
