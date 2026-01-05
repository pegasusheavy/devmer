// Service interfaces for DI

export interface IConfigService {
  get<T>(key: string): T | undefined;
  getOrThrow<T>(key: string): T;
  isDevelopment(): boolean;
  isProduction(): boolean;
}

import type { PrismaClient } from '@prisma/client';

export interface IDatabaseService {
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  isConnected(): boolean;
  getClient(): PrismaClient;
}

export interface IEmailService {
  send(options: EmailOptions): Promise<EmailResult>;
  sendTemplate(template: string, to: string, data: Record<string, unknown>): Promise<EmailResult>;
  sendBulk(options: BulkEmailOptions): Promise<BulkEmailResult>;
}

export interface IAnalyticsService {
  trackPageView(data: PageViewData): Promise<void>;
  trackEvent(data: EventData): Promise<void>;
  getPageViews(filter: AnalyticsFilter): Promise<PageViewData[]>;
  getEvents(filter: AnalyticsFilter): Promise<EventData[]>;
}

export interface ILeadService {
  create(data: CreateLeadData): Promise<Lead>;
  findById(id: string): Promise<Lead | null>;
  findByEmail(email: string): Promise<Lead | null>;
  update(id: string, data: UpdateLeadData): Promise<Lead>;
  updateScore(id: string, score: number): Promise<Lead>;
  recordActivity(leadId: string, activity: LeadActivityData): Promise<void>;
  list(filter: LeadFilter): Promise<PaginatedResult<Lead>>;
}

export interface INewsletterService {
  subscribe(email: string, preferences?: NewsletterPreferences): Promise<Subscriber>;
  unsubscribe(email: string): Promise<void>;
  confirm(email: string, token: string): Promise<void>;
  updatePreferences(email: string, preferences: NewsletterPreferences): Promise<Subscriber>;
  getSubscribers(filter: SubscriberFilter): Promise<PaginatedResult<Subscriber>>;
}

export interface IBlogService {
  create(data: CreatePostData): Promise<BlogPost>;
  findBySlug(slug: string): Promise<BlogPost | null>;
  findById(id: string): Promise<BlogPost | null>;
  update(id: string, data: UpdatePostData): Promise<BlogPost>;
  publish(id: string): Promise<BlogPost>;
  unpublish(id: string): Promise<BlogPost>;
  delete(id: string): Promise<void>;
  list(filter: PostFilter): Promise<PaginatedResult<BlogPost>>;
}

export interface IContactService {
  submit(data: ContactSubmissionData): Promise<ContactSubmission>;
  findById(id: string): Promise<ContactSubmission | null>;
  respond(id: string, response: ContactResponseData): Promise<ContactSubmission>;
  list(filter: ContactFilter): Promise<PaginatedResult<ContactSubmission>>;
}

export interface ICacheService {
  get<T>(key: string): Promise<T | null>;
  set<T>(key: string, value: T, ttlSeconds?: number): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}

// Data types

export interface EmailOptions {
  to: string | string[];
  subject: string;
  html?: string;
  text?: string;
  from?: string;
  replyTo?: string;
}

export interface EmailResult {
  success: boolean;
  messageId?: string;
  error?: string;
}

export interface BulkEmailOptions {
  emails: EmailOptions[];
  batchSize?: number;
}

export interface BulkEmailResult {
  total: number;
  sent: number;
  failed: number;
  errors: Array<{ email: string; error: string }>;
}

export interface PageViewData {
  path: string;
  referrer?: string;
  userAgent?: string;
  ipAddress?: string;
  sessionId?: string;
  visitorId?: string;
  utmSource?: string;
  utmMedium?: string;
  utmCampaign?: string;
}

export interface EventData {
  name: string;
  category?: string;
  properties?: Record<string, unknown>;
  sessionId?: string;
  visitorId?: string;
  path?: string;
}

export interface AnalyticsFilter {
  startDate?: Date;
  endDate?: Date;
  path?: string;
  limit?: number;
  offset?: number;
}

export interface Lead {
  id: string;
  email: string;
  firstName?: string;
  lastName?: string;
  company?: string;
  jobTitle?: string;
  score: number;
  status: string;
  source?: string;
  createdAt: Date;
  updatedAt: Date;
}

export interface CreateLeadData {
  email: string;
  firstName?: string;
  lastName?: string;
  company?: string;
  jobTitle?: string;
  source?: string;
  campaign?: string;
}

export interface UpdateLeadData {
  firstName?: string;
  lastName?: string;
  company?: string;
  jobTitle?: string;
  status?: string;
  score?: number;
}

export interface LeadActivityData {
  type: string;
  description?: string;
  metadata?: Record<string, unknown>;
}

export interface LeadFilter {
  status?: string;
  source?: string;
  minScore?: number;
  search?: string;
  limit?: number;
  offset?: number;
}

export interface Subscriber {
  id: string;
  email: string;
  firstName?: string;
  status: string;
  preferences?: NewsletterPreferences;
  createdAt: Date;
}

export interface NewsletterPreferences {
  productUpdates?: boolean;
  blog?: boolean;
  events?: boolean;
  weeklyDigest?: boolean;
}

export interface SubscriberFilter {
  status?: string;
  limit?: number;
  offset?: number;
}

export interface BlogPost {
  id: string;
  slug: string;
  title: string;
  excerpt?: string;
  content: string;
  coverImage?: string;
  tags: string[];
  category?: string;
  status: string;
  featured: boolean;
  publishedAt?: Date;
  createdAt: Date;
  updatedAt: Date;
  author: {
    id: string;
    name?: string;
  };
}

export interface CreatePostData {
  slug: string;
  title: string;
  excerpt?: string;
  content: string;
  coverImage?: string;
  tags?: string[];
  category?: string;
  authorId: string;
}

export interface UpdatePostData {
  title?: string;
  excerpt?: string;
  content?: string;
  coverImage?: string;
  tags?: string[];
  category?: string;
  metaTitle?: string;
  metaDescription?: string;
}

export interface PostFilter {
  status?: string;
  category?: string;
  tag?: string;
  featured?: boolean;
  authorId?: string;
  search?: string;
  limit?: number;
  offset?: number;
}

export interface ContactSubmission {
  id: string;
  email: string;
  name: string;
  company?: string;
  subject?: string;
  message: string;
  type: string;
  status: string;
  createdAt: Date;
}

export interface ContactSubmissionData {
  email: string;
  name: string;
  company?: string;
  subject?: string;
  message: string;
  type?: string;
  ipAddress?: string;
  userAgent?: string;
  referrer?: string;
}

export interface ContactResponseData {
  respondedBy: string;
  notes?: string;
}

export interface ContactFilter {
  status?: string;
  type?: string;
  search?: string;
  limit?: number;
  offset?: number;
}

export interface PaginatedResult<T> {
  data: T[];
  total: number;
  limit: number;
  offset: number;
  hasMore: boolean;
}
