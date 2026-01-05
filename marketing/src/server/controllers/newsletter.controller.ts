import { Hono } from 'hono';
import { resolve, TOKENS } from '../container';
import type { INewsletterService } from '../types';

export const newsletterController = new Hono();

// Subscribe to newsletter
newsletterController.post('/subscribe', async (c) => {
  const newsletter = resolve<INewsletterService>(TOKENS.Newsletter);
  const { email, preferences } = await c.req.json();

  if (!email) {
    return c.json({ error: 'Email is required' }, 400);
  }

  try {
    const subscriber = await newsletter.subscribe(email, preferences);
    return c.json({ success: true, subscriber });
  } catch (error) {
    console.error('Newsletter subscribe error:', error);
    return c.json({ error: 'Failed to subscribe' }, 500);
  }
});

// Confirm subscription
newsletterController.get('/confirm', async (c) => {
  const newsletter = resolve<INewsletterService>(TOKENS.Newsletter);
  const email = c.req.query('email');
  const token = c.req.query('token');

  if (!email || !token) {
    return c.json({ error: 'Missing email or token' }, 400);
  }

  try {
    await newsletter.confirm(email, token);
    return c.redirect('/?subscribed=true');
  } catch (error) {
    console.error('Newsletter confirm error:', error);
    return c.json({ error: 'Failed to confirm subscription' }, 500);
  }
});

// Unsubscribe
newsletterController.post('/unsubscribe', async (c) => {
  const newsletter = resolve<INewsletterService>(TOKENS.Newsletter);
  const { email } = await c.req.json();

  if (!email) {
    return c.json({ error: 'Email is required' }, 400);
  }

  try {
    await newsletter.unsubscribe(email);
    return c.json({ success: true });
  } catch (error) {
    console.error('Newsletter unsubscribe error:', error);
    return c.json({ error: 'Failed to unsubscribe' }, 500);
  }
});

// Update preferences
newsletterController.put('/preferences', async (c) => {
  const newsletter = resolve<INewsletterService>(TOKENS.Newsletter);
  const { email, preferences } = await c.req.json();

  if (!email) {
    return c.json({ error: 'Email is required' }, 400);
  }

  try {
    const subscriber = await newsletter.updatePreferences(email, preferences);
    return c.json({ success: true, subscriber });
  } catch (error) {
    console.error('Newsletter preferences error:', error);
    return c.json({ error: 'Failed to update preferences' }, 500);
  }
});
