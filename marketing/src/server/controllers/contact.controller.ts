import { Hono } from 'hono';
import { resolve, TOKENS } from '../container';
import type { IContactService } from '../types';

export const contactController = new Hono();

// Submit contact form
contactController.post('/', async (c) => {
  const contact = resolve<IContactService>(TOKENS.Contact);

  const body = await c.req.json();
  const { email, name, company, subject, message, type } = body;

  if (!email || !name || !message) {
    return c.json({ error: 'Email, name, and message are required' }, 400);
  }

  try {
    const submission = await contact.submit({
      email,
      name,
      company,
      subject,
      message,
      type,
      ipAddress: c.req.header('x-forwarded-for') || c.req.header('x-real-ip'),
      userAgent: c.req.header('user-agent'),
      referrer: c.req.header('referer'),
    });

    return c.json({ success: true, id: submission.id });
  } catch (error) {
    console.error('Contact submit error:', error);
    return c.json({ error: 'Failed to submit contact form' }, 500);
  }
});

// Get submission (admin)
contactController.get('/:id', async (c) => {
  const contact = resolve<IContactService>(TOKENS.Contact);
  const id = c.req.param('id');

  try {
    const submission = await contact.findById(id);
    if (!submission) {
      return c.json({ error: 'Submission not found' }, 404);
    }
    return c.json(submission);
  } catch (error) {
    console.error('Contact get error:', error);
    return c.json({ error: 'Failed to get submission' }, 500);
  }
});

// List submissions (admin)
contactController.get('/', async (c) => {
  const contact = resolve<IContactService>(TOKENS.Contact);

  const status = c.req.query('status');
  const type = c.req.query('type');
  const search = c.req.query('search');
  const limit = parseInt(c.req.query('limit') || '20', 10);
  const offset = parseInt(c.req.query('offset') || '0', 10);

  try {
    const result = await contact.list({ status, type, search, limit, offset });
    return c.json(result);
  } catch (error) {
    console.error('Contact list error:', error);
    return c.json({ error: 'Failed to list submissions' }, 500);
  }
});
