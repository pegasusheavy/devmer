import { Injectable, Inject, PLATFORM_ID } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';
import { DOCUMENT, isPlatformBrowser } from '@angular/common';
import { Router, NavigationEnd } from '@angular/router';
import { filter } from 'rxjs/operators';

/**
 * Page metadata for SEO
 */
export interface PageMeta {
  title: string;
  description: string;
  keywords?: string[];
  canonical?: string;
  ogType?: 'website' | 'article' | 'product';
  ogImage?: string;
  twitterCard?: 'summary' | 'summary_large_image';
  noIndex?: boolean;
  article?: {
    publishedTime?: string;
    modifiedTime?: string;
    author?: string;
    section?: string;
    tags?: string[];
  };
}

/**
 * Structured data types for JSON-LD
 */
export interface BreadcrumbItem {
  name: string;
  url: string;
}

export interface ArticleData {
  headline: string;
  description: string;
  image: string;
  datePublished: string;
  dateModified?: string;
  author: {
    name: string;
    url?: string;
  };
}

export interface HowToStep {
  name: string;
  text: string;
  image?: string;
  url?: string;
}

export interface HowToData {
  name: string;
  description: string;
  totalTime?: string;
  steps: HowToStep[];
}

export interface FAQItem {
  question: string;
  answer: string;
}

/**
 * SEO Service for Angular
 * 
 * Manages meta tags, structured data, and AEO optimizations
 */
@Injectable({
  providedIn: 'root'
})
export class SeoService {
  private readonly baseUrl = 'https://devmer.io';
  private readonly siteName = 'Devmer';
  private readonly defaultImage = `${this.baseUrl}/assets/og-image.png`;
  private readonly twitterHandle = '@PegasusHeavyInd';

  constructor(
    private meta: Meta,
    private titleService: Title,
    private router: Router,
    @Inject(DOCUMENT) private document: Document,
    @Inject(PLATFORM_ID) private platformId: object
  ) {
    // Track route changes for analytics
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd)
    ).subscribe(() => {
      this.updateCanonicalUrl();
    });
  }

  /**
   * Update page metadata
   */
  updatePageMeta(meta: PageMeta): void {
    // Title
    const fullTitle = meta.title.includes(this.siteName) 
      ? meta.title 
      : `${meta.title} | ${this.siteName}`;
    this.titleService.setTitle(fullTitle);

    // Primary meta tags
    this.meta.updateTag({ name: 'title', content: fullTitle });
    this.meta.updateTag({ name: 'description', content: meta.description });
    
    if (meta.keywords?.length) {
      this.meta.updateTag({ name: 'keywords', content: meta.keywords.join(', ') });
    }

    // Robots
    if (meta.noIndex) {
      this.meta.updateTag({ name: 'robots', content: 'noindex, nofollow' });
    } else {
      this.meta.updateTag({ name: 'robots', content: 'index, follow' });
    }

    // Canonical URL
    const canonical = meta.canonical || `${this.baseUrl}${this.router.url.split('?')[0]}`;
    this.updateCanonicalUrl(canonical);

    // Open Graph
    this.meta.updateTag({ property: 'og:type', content: meta.ogType || 'website' });
    this.meta.updateTag({ property: 'og:title', content: meta.title });
    this.meta.updateTag({ property: 'og:description', content: meta.description });
    this.meta.updateTag({ property: 'og:url', content: canonical });
    this.meta.updateTag({ property: 'og:image', content: meta.ogImage || this.defaultImage });
    this.meta.updateTag({ property: 'og:site_name', content: this.siteName });

    // Twitter
    this.meta.updateTag({ name: 'twitter:card', content: meta.twitterCard || 'summary_large_image' });
    this.meta.updateTag({ name: 'twitter:title', content: meta.title });
    this.meta.updateTag({ name: 'twitter:description', content: meta.description });
    this.meta.updateTag({ name: 'twitter:image', content: meta.ogImage || this.defaultImage });
    this.meta.updateTag({ name: 'twitter:creator', content: this.twitterHandle });

    // Article-specific meta
    if (meta.article) {
      this.meta.updateTag({ property: 'og:type', content: 'article' });
      
      if (meta.article.publishedTime) {
        this.meta.updateTag({ property: 'article:published_time', content: meta.article.publishedTime });
      }
      if (meta.article.modifiedTime) {
        this.meta.updateTag({ property: 'article:modified_time', content: meta.article.modifiedTime });
      }
      if (meta.article.author) {
        this.meta.updateTag({ property: 'article:author', content: meta.article.author });
      }
      if (meta.article.section) {
        this.meta.updateTag({ property: 'article:section', content: meta.article.section });
      }
      if (meta.article.tags?.length) {
        meta.article.tags.forEach(tag => {
          this.meta.addTag({ property: 'article:tag', content: tag });
        });
      }
    }
  }

  /**
   * Update canonical URL
   */
  updateCanonicalUrl(url?: string): void {
    if (!isPlatformBrowser(this.platformId)) return;

    const canonical = url || `${this.baseUrl}${this.router.url.split('?')[0]}`;
    
    // Remove existing canonical
    const existing = this.document.querySelector('link[rel="canonical"]');
    if (existing) {
      existing.remove();
    }

    // Add new canonical
    const link = this.document.createElement('link');
    link.setAttribute('rel', 'canonical');
    link.setAttribute('href', canonical);
    this.document.head.appendChild(link);
  }

  /**
   * Add breadcrumb structured data
   */
  addBreadcrumbs(items: BreadcrumbItem[]): void {
    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'BreadcrumbList',
      itemListElement: items.map((item, index) => ({
        '@type': 'ListItem',
        position: index + 1,
        name: item.name,
        item: item.url.startsWith('http') ? item.url : `${this.baseUrl}${item.url}`
      }))
    };

    this.addJsonLd('breadcrumb', structuredData);
  }

  /**
   * Add article structured data
   */
  addArticleStructuredData(article: ArticleData): void {
    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'Article',
      headline: article.headline,
      description: article.description,
      image: article.image,
      datePublished: article.datePublished,
      dateModified: article.dateModified || article.datePublished,
      author: {
        '@type': 'Person',
        name: article.author.name,
        url: article.author.url
      },
      publisher: {
        '@type': 'Organization',
        name: 'Pegasus Heavy Industries',
        logo: {
          '@type': 'ImageObject',
          url: `${this.baseUrl}/assets/logo.png`
        }
      },
      mainEntityOfPage: {
        '@type': 'WebPage',
        '@id': `${this.baseUrl}${this.router.url}`
      }
    };

    this.addJsonLd('article', structuredData);
  }

  /**
   * Add HowTo structured data (AEO optimization)
   */
  addHowToStructuredData(howTo: HowToData): void {
    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'HowTo',
      name: howTo.name,
      description: howTo.description,
      totalTime: howTo.totalTime,
      step: howTo.steps.map((step, index) => ({
        '@type': 'HowToStep',
        position: index + 1,
        name: step.name,
        text: step.text,
        image: step.image,
        url: step.url
      }))
    };

    this.addJsonLd('howto', structuredData);
  }

  /**
   * Add FAQ structured data (AEO optimization)
   */
  addFAQStructuredData(faqs: FAQItem[]): void {
    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'FAQPage',
      mainEntity: faqs.map(faq => ({
        '@type': 'Question',
        name: faq.question,
        acceptedAnswer: {
          '@type': 'Answer',
          text: faq.answer
        }
      }))
    };

    this.addJsonLd('faq', structuredData);
  }

  /**
   * Add product/pricing structured data
   */
  addProductStructuredData(product: {
    name: string;
    description: string;
    price: string | number;
    priceCurrency?: string;
    availability?: 'InStock' | 'OutOfStock' | 'PreOrder';
  }): void {
    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'Product',
      name: product.name,
      description: product.description,
      brand: {
        '@type': 'Organization',
        name: 'Pegasus Heavy Industries'
      },
      offers: {
        '@type': 'Offer',
        price: product.price,
        priceCurrency: product.priceCurrency || 'USD',
        availability: `https://schema.org/${product.availability || 'InStock'}`
      }
    };

    this.addJsonLd('product', structuredData);
  }

  /**
   * Add video structured data
   */
  addVideoStructuredData(video: {
    name: string;
    description: string;
    thumbnailUrl: string;
    uploadDate: string;
    duration?: string;
    embedUrl?: string;
  }): void {
    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'VideoObject',
      name: video.name,
      description: video.description,
      thumbnailUrl: video.thumbnailUrl,
      uploadDate: video.uploadDate,
      duration: video.duration,
      embedUrl: video.embedUrl,
      publisher: {
        '@type': 'Organization',
        name: 'Pegasus Heavy Industries',
        logo: {
          '@type': 'ImageObject',
          url: `${this.baseUrl}/assets/logo.png`
        }
      }
    };

    this.addJsonLd('video', structuredData);
  }

  /**
   * Add JSON-LD structured data to the page
   */
  private addJsonLd(id: string, data: object): void {
    if (!isPlatformBrowser(this.platformId)) return;

    // Remove existing script with same ID
    const existing = this.document.getElementById(`json-ld-${id}`);
    if (existing) {
      existing.remove();
    }

    // Create new script element
    const script = this.document.createElement('script');
    script.id = `json-ld-${id}`;
    script.type = 'application/ld+json';
    script.text = JSON.stringify(data);
    this.document.head.appendChild(script);
  }

  /**
   * Remove all dynamic JSON-LD scripts
   */
  clearStructuredData(): void {
    if (!isPlatformBrowser(this.platformId)) return;

    const scripts = this.document.querySelectorAll('script[id^="json-ld-"]');
    scripts.forEach(script => script.remove());
  }

  /**
   * Generate hreflang tags for internationalization
   */
  addHreflangTags(alternates: { lang: string; url: string }[]): void {
    if (!isPlatformBrowser(this.platformId)) return;

    // Remove existing hreflang tags
    const existing = this.document.querySelectorAll('link[hreflang]');
    existing.forEach(el => el.remove());

    // Add new hreflang tags
    alternates.forEach(alt => {
      const link = this.document.createElement('link');
      link.setAttribute('rel', 'alternate');
      link.setAttribute('hreflang', alt.lang);
      link.setAttribute('href', alt.url);
      this.document.head.appendChild(link);
    });

    // Add x-default
    const xDefault = this.document.createElement('link');
    xDefault.setAttribute('rel', 'alternate');
    xDefault.setAttribute('hreflang', 'x-default');
    xDefault.setAttribute('href', `${this.baseUrl}${this.router.url}`);
    this.document.head.appendChild(xDefault);
  }
}
