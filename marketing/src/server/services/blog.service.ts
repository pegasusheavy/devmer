import { injectable, inject } from 'tsyringe';
import type {
  IBlogService,
  ICacheService,
  BlogPost,
  CreatePostData,
  UpdatePostData,
  PostFilter,
  PaginatedResult,
} from '../types';
import { TOKENS } from '../container';
import type { PrismaService } from './prisma.service';

@injectable()
export class BlogService implements IBlogService {
  private readonly CACHE_PREFIX = 'blog:';
  private readonly CACHE_TTL = 300; // 5 minutes

  constructor(
    @inject(TOKENS.Prisma) private prisma: PrismaService,
    @inject(TOKENS.Cache) private cache: ICacheService
  ) {}

  async create(data: CreatePostData): Promise<BlogPost> {
    const client = this.prisma.client;

    const post = await client.blogPost.create({
      data: {
        slug: data.slug,
        title: data.title,
        excerpt: data.excerpt,
        content: data.content,
        coverImage: data.coverImage,
        tags: data.tags || [],
        category: data.category,
        authorId: data.authorId,
        status: 'DRAFT',
      },
      include: { author: true },
    });

    return this.mapPost(post);
  }

  async findBySlug(slug: string): Promise<BlogPost | null> {
    // Check cache first
    const cached = await this.cache.get<BlogPost>(`${this.CACHE_PREFIX}slug:${slug}`);
    if (cached) {
      return cached;
    }

    const client = this.prisma.client;
    const post = await client.blogPost.findUnique({
      where: { slug },
      include: { author: true },
    });

    if (!post) {
      return null;
    }

    const mapped = this.mapPost(post);

    // Cache if published
    if (post.status === 'PUBLISHED') {
      await this.cache.set(`${this.CACHE_PREFIX}slug:${slug}`, mapped, this.CACHE_TTL);
    }

    return mapped;
  }

  async findById(id: string): Promise<BlogPost | null> {
    const client = this.prisma.client;
    const post = await client.blogPost.findUnique({
      where: { id },
      include: { author: true },
    });

    return post ? this.mapPost(post) : null;
  }

  async update(id: string, data: UpdatePostData): Promise<BlogPost> {
    const client = this.prisma.client;

    const post = await client.blogPost.update({
      where: { id },
      data: {
        title: data.title,
        excerpt: data.excerpt,
        content: data.content,
        coverImage: data.coverImage,
        tags: data.tags,
        category: data.category,
        metaTitle: data.metaTitle,
        metaDescription: data.metaDescription,
      },
      include: { author: true },
    });

    // Invalidate cache
    await this.cache.delete(`${this.CACHE_PREFIX}slug:${post.slug}`);
    await this.cache.delete(`${this.CACHE_PREFIX}id:${id}`);

    return this.mapPost(post);
  }

  async publish(id: string): Promise<BlogPost> {
    const client = this.prisma.client;

    const post = await client.blogPost.update({
      where: { id },
      data: {
        status: 'PUBLISHED',
        publishedAt: new Date(),
      },
      include: { author: true },
    });

    return this.mapPost(post);
  }

  async unpublish(id: string): Promise<BlogPost> {
    const client = this.prisma.client;

    const post = await client.blogPost.update({
      where: { id },
      data: {
        status: 'DRAFT',
      },
      include: { author: true },
    });

    // Invalidate cache
    await this.cache.delete(`${this.CACHE_PREFIX}slug:${post.slug}`);

    return this.mapPost(post);
  }

  async delete(id: string): Promise<void> {
    const client = this.prisma.client;

    const post = await client.blogPost.findUnique({ where: { id } });
    if (post) {
      await this.cache.delete(`${this.CACHE_PREFIX}slug:${post.slug}`);
    }

    await client.blogPost.delete({ where: { id } });
  }

  async list(filter: PostFilter): Promise<PaginatedResult<BlogPost>> {
    const client = this.prisma.client;
    const limit = filter.limit || 10;
    const offset = filter.offset || 0;

    const where: Record<string, unknown> = {};

    if (filter.status) {
      where['status'] = filter.status;
    }

    if (filter.category) {
      where['category'] = filter.category;
    }

    if (filter.tag) {
      where['tags'] = { has: filter.tag };
    }

    if (filter.featured !== undefined) {
      where['featured'] = filter.featured;
    }

    if (filter.authorId) {
      where['authorId'] = filter.authorId;
    }

    if (filter.search) {
      where['OR'] = [
        { title: { contains: filter.search, mode: 'insensitive' } },
        { excerpt: { contains: filter.search, mode: 'insensitive' } },
        { content: { contains: filter.search, mode: 'insensitive' } },
      ];
    }

    const [posts, total] = await Promise.all([
      client.blogPost.findMany({
        where,
        take: limit,
        skip: offset,
        orderBy: { publishedAt: 'desc' },
        include: { author: true },
      }),
      client.blogPost.count({ where }),
    ]);

    return {
      data: posts.map(this.mapPost),
      total,
      limit,
      offset,
      hasMore: offset + posts.length < total,
    };
  }

  private mapPost(post: Record<string, unknown>): BlogPost {
    const author = post['author'] as Record<string, unknown>;
    return {
      id: post['id'] as string,
      slug: post['slug'] as string,
      title: post['title'] as string,
      excerpt: post['excerpt'] as string | undefined,
      content: post['content'] as string,
      coverImage: post['coverImage'] as string | undefined,
      tags: post['tags'] as string[],
      category: post['category'] as string | undefined,
      status: post['status'] as string,
      featured: post['featured'] as boolean,
      publishedAt: post['publishedAt'] as Date | undefined,
      createdAt: post['createdAt'] as Date,
      updatedAt: post['updatedAt'] as Date,
      author: {
        id: author['id'] as string,
        name: author['name'] as string | undefined,
      },
    };
  }
}
