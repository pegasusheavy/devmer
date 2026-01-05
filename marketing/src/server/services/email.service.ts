import { injectable, inject } from 'tsyringe';
import type {
  IEmailService,
  IConfigService,
  EmailOptions,
  EmailResult,
  BulkEmailOptions,
  BulkEmailResult,
} from '../types';
import { TOKENS } from '../container';

@injectable()
export class EmailService implements IEmailService {
  private templates = new Map<string, (data: Record<string, unknown>) => { subject: string; html: string }>();

  constructor(
    @inject(TOKENS.Config) private configService: IConfigService
  ) {
    this.registerDefaultTemplates();
  }

  private registerDefaultTemplates(): void {
    // Welcome email
    this.templates.set('welcome', (data) => ({
      subject: `Welcome to ${this.configService.get<string>('app.name')}!`,
      html: `
        <h1>Welcome, ${data['name'] || 'there'}!</h1>
        <p>Thanks for signing up. We're excited to have you on board.</p>
        <p>Get started by exploring our documentation or requesting a demo.</p>
      `,
    }));

    // Newsletter confirmation
    this.templates.set('newsletter-confirm', (data) => ({
      subject: 'Confirm your subscription',
      html: `
        <h1>Confirm your subscription</h1>
        <p>Click the link below to confirm your newsletter subscription:</p>
        <a href="${data['confirmUrl']}">Confirm Subscription</a>
      `,
    }));

    // Contact form acknowledgment
    this.templates.set('contact-received', (data) => ({
      subject: 'We received your message',
      html: `
        <h1>Thanks for reaching out!</h1>
        <p>Hi ${data['name']},</p>
        <p>We've received your message and will get back to you within 24-48 hours.</p>
        <p>Your message:</p>
        <blockquote>${data['message']}</blockquote>
      `,
    }));

    // Demo request confirmation
    this.templates.set('demo-scheduled', (data) => ({
      subject: 'Your demo is scheduled!',
      html: `
        <h1>Demo Confirmed</h1>
        <p>Hi ${data['name']},</p>
        <p>Your demo has been scheduled for ${data['date']} at ${data['time']}.</p>
        <p>You'll receive a calendar invite shortly.</p>
      `,
    }));
  }

  async send(options: EmailOptions): Promise<EmailResult> {
    const host = this.configService.get<string>('email.host');

    if (!host) {
      // Log in development, fail in production
      if (this.configService.isDevelopment()) {
        console.log('📧 [DEV] Email would be sent:', {
          to: options.to,
          subject: options.subject,
        });
        return { success: true, messageId: 'dev-' + Date.now() };
      }
      return { success: false, error: 'Email not configured' };
    }

    try {
      // In a real implementation, you would use nodemailer or similar
      // For now, we'll simulate sending
      console.log('📧 Sending email:', {
        to: options.to,
        subject: options.subject,
        from: options.from || this.configService.get<string>('email.from'),
      });

      // Simulate network delay
      await new Promise((resolve) => setTimeout(resolve, 100));

      return {
        success: true,
        messageId: `msg-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async sendTemplate(
    template: string,
    to: string,
    data: Record<string, unknown>
  ): Promise<EmailResult> {
    const templateFn = this.templates.get(template);

    if (!templateFn) {
      return { success: false, error: `Template "${template}" not found` };
    }

    const { subject, html } = templateFn(data);

    return this.send({ to, subject, html });
  }

  async sendBulk(options: BulkEmailOptions): Promise<BulkEmailResult> {
    const batchSize = options.batchSize || 10;
    const results: BulkEmailResult = {
      total: options.emails.length,
      sent: 0,
      failed: 0,
      errors: [],
    };

    // Process in batches
    for (let i = 0; i < options.emails.length; i += batchSize) {
      const batch = options.emails.slice(i, i + batchSize);

      const batchResults = await Promise.all(
        batch.map(async (email) => {
          const result = await this.send(email);
          return { email: Array.isArray(email.to) ? email.to[0] : email.to, result };
        })
      );

      for (const { email, result } of batchResults) {
        if (result.success) {
          results.sent++;
        } else {
          results.failed++;
          results.errors.push({ email, error: result.error || 'Unknown error' });
        }
      }

      // Small delay between batches
      if (i + batchSize < options.emails.length) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    }

    return results;
  }
}
