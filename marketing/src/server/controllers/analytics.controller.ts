import { Hono } from 'hono';
import { resolve, TOKENS } from '../container';
import type { IAnalyticsService } from '../types';

export const analyticsController = new Hono();

// Track page view
analyticsController.post('/pageview', async (c) => {
  const analytics = resolve<IAnalyticsService>(TOKENS.Analytics);

  const body = await c.req.json();
  const { path, sessionId, visitorId, utmSource, utmMedium, utmCampaign } = body;

  if (!path) {
    return c.json({ error: 'Path is required' }, 400);
  }

  try {
    await analytics.trackPageView({
      path,
      referrer: c.req.header('referer'),
      userAgent: c.req.header('user-agent'),
      ipAddress: c.req.header('x-forwarded-for') || c.req.header('x-real-ip'),
      sessionId,
      visitorId,
      utmSource,
      utmMedium,
      utmCampaign,
    });
    return c.json({ success: true });
  } catch (error) {
    console.error('Analytics pageview error:', error);
    return c.json({ error: 'Failed to track page view' }, 500);
  }
});

// Track event
analyticsController.post('/event', async (c) => {
  const analytics = resolve<IAnalyticsService>(TOKENS.Analytics);

  const body = await c.req.json();
  const { name, category, properties, sessionId, visitorId, path } = body;

  if (!name) {
    return c.json({ error: 'Event name is required' }, 400);
  }

  try {
    await analytics.trackEvent({
      name,
      category,
      properties,
      sessionId,
      visitorId,
      path,
    });
    return c.json({ success: true });
  } catch (error) {
    console.error('Analytics event error:', error);
    return c.json({ error: 'Failed to track event' }, 500);
  }
});
