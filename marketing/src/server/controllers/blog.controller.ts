import { Hono } from 'hono';
import { resolve, TOKENS } from '../container';
import type { IBlogService } from '../types';

export const blogController = new Hono();

// List published posts
blogController.get('/', async (c) => {
  const blog = resolve<IBlogService>(TOKENS.Blog);

  const category = c.req.query('category');
  const tag = c.req.query('tag');
  const featured = c.req.query('featured') === 'true' ? true : undefined;
  const search = c.req.query('search');
  const limit = parseInt(c.req.query('limit') || '10', 10);
  const offset = parseInt(c.req.query('offset') || '0', 10);

  try {
    const result = await blog.list({
      status: 'PUBLISHED',
      category,
      tag,
      featured,
      search,
      limit,
      offset,
    });
    return c.json(result);
  } catch (error) {
    console.error('Blog list error:', error);
    return c.json({ error: 'Failed to list posts' }, 500);
  }
});

// Get post by slug
blogController.get('/:slug', async (c) => {
  const blog = resolve<IBlogService>(TOKENS.Blog);
  const slug = c.req.param('slug');

  try {
    const post = await blog.findBySlug(slug);
    if (!post) {
      return c.json({ error: 'Post not found' }, 404);
    }

    // Only return published posts to public
    if (post.status !== 'PUBLISHED') {
      return c.json({ error: 'Post not found' }, 404);
    }

    return c.json(post);
  } catch (error) {
    console.error('Blog get error:', error);
    return c.json({ error: 'Failed to get post' }, 500);
  }
});

// Get featured posts
blogController.get('/featured', async (c) => {
  const blog = resolve<IBlogService>(TOKENS.Blog);

  try {
    const result = await blog.list({
      status: 'PUBLISHED',
      featured: true,
      limit: 5,
    });
    return c.json(result.data);
  } catch (error) {
    console.error('Blog featured error:', error);
    return c.json({ error: 'Failed to get featured posts' }, 500);
  }
});
