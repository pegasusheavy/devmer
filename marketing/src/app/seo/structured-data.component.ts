import { Component, Input, OnChanges, OnDestroy, Inject, PLATFORM_ID } from '@angular/core';
import { DOCUMENT, isPlatformBrowser } from '@angular/common';

/**
 * Component for injecting JSON-LD structured data
 * 
 * @example
 * ```html
 * <app-structured-data [type]="'Article'" [data]="articleData"></app-structured-data>
 * ```
 */
@Component({
  selector: 'app-structured-data',
  standalone: true,
  template: ''
})
export class StructuredDataComponent implements OnChanges, OnDestroy {
  @Input() type!: string;
  @Input() data!: Record<string, unknown>;
  @Input() id?: string;

  private scriptElement: HTMLScriptElement | null = null;

  constructor(
    @Inject(DOCUMENT) private document: Document,
    @Inject(PLATFORM_ID) private platformId: object
  ) {}

  ngOnChanges(): void {
    this.updateStructuredData();
  }

  ngOnDestroy(): void {
    this.removeStructuredData();
  }

  private updateStructuredData(): void {
    if (!isPlatformBrowser(this.platformId) || !this.type || !this.data) {
      return;
    }

    // Remove existing script
    this.removeStructuredData();

    // Create structured data object
    const structuredData = {
      '@context': 'https://schema.org',
      '@type': this.type,
      ...this.data
    };

    // Create script element
    this.scriptElement = this.document.createElement('script');
    this.scriptElement.type = 'application/ld+json';
    this.scriptElement.id = `structured-data-${this.id || this.type.toLowerCase()}`;
    this.scriptElement.text = JSON.stringify(structuredData);

    // Append to head
    this.document.head.appendChild(this.scriptElement);
  }

  private removeStructuredData(): void {
    if (this.scriptElement) {
      this.scriptElement.remove();
      this.scriptElement = null;
    }
  }
}

/**
 * Pre-built Article structured data component
 */
@Component({
  selector: 'app-article-structured-data',
  standalone: true,
  template: ''
})
export class ArticleStructuredDataComponent implements OnChanges, OnDestroy {
  @Input() headline!: string;
  @Input() description!: string;
  @Input() image!: string;
  @Input() datePublished!: string;
  @Input() dateModified?: string;
  @Input() authorName!: string;
  @Input() authorUrl?: string;

  private scriptElement: HTMLScriptElement | null = null;

  constructor(
    @Inject(DOCUMENT) private document: Document,
    @Inject(PLATFORM_ID) private platformId: object
  ) {}

  ngOnChanges(): void {
    this.updateStructuredData();
  }

  ngOnDestroy(): void {
    this.removeStructuredData();
  }

  private updateStructuredData(): void {
    if (!isPlatformBrowser(this.platformId) || !this.headline) {
      return;
    }

    this.removeStructuredData();

    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'Article',
      headline: this.headline,
      description: this.description,
      image: this.image,
      datePublished: this.datePublished,
      dateModified: this.dateModified || this.datePublished,
      author: {
        '@type': 'Person',
        name: this.authorName,
        url: this.authorUrl
      },
      publisher: {
        '@type': 'Organization',
        name: 'Joseph R. Quinn',
        logo: {
          '@type': 'ImageObject',
          url: 'https://devmer.io/assets/logo.png'
        }
      }
    };

    this.scriptElement = this.document.createElement('script');
    this.scriptElement.type = 'application/ld+json';
    this.scriptElement.id = 'structured-data-article';
    this.scriptElement.text = JSON.stringify(structuredData);
    this.document.head.appendChild(this.scriptElement);
  }

  private removeStructuredData(): void {
    if (this.scriptElement) {
      this.scriptElement.remove();
      this.scriptElement = null;
    }
  }
}

/**
 * Pre-built FAQ structured data component for AEO
 */
@Component({
  selector: 'app-faq-structured-data',
  standalone: true,
  template: ''
})
export class FAQStructuredDataComponent implements OnChanges, OnDestroy {
  @Input() faqs: Array<{ question: string; answer: string }> = [];

  private scriptElement: HTMLScriptElement | null = null;

  constructor(
    @Inject(DOCUMENT) private document: Document,
    @Inject(PLATFORM_ID) private platformId: object
  ) {}

  ngOnChanges(): void {
    this.updateStructuredData();
  }

  ngOnDestroy(): void {
    this.removeStructuredData();
  }

  private updateStructuredData(): void {
    if (!isPlatformBrowser(this.platformId) || !this.faqs.length) {
      return;
    }

    this.removeStructuredData();

    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'FAQPage',
      mainEntity: this.faqs.map(faq => ({
        '@type': 'Question',
        name: faq.question,
        acceptedAnswer: {
          '@type': 'Answer',
          text: faq.answer
        }
      }))
    };

    this.scriptElement = this.document.createElement('script');
    this.scriptElement.type = 'application/ld+json';
    this.scriptElement.id = 'structured-data-faq';
    this.scriptElement.text = JSON.stringify(structuredData);
    this.document.head.appendChild(this.scriptElement);
  }

  private removeStructuredData(): void {
    if (this.scriptElement) {
      this.scriptElement.remove();
      this.scriptElement = null;
    }
  }
}

/**
 * Pre-built HowTo structured data component for AEO
 */
@Component({
  selector: 'app-howto-structured-data',
  standalone: true,
  template: ''
})
export class HowToStructuredDataComponent implements OnChanges, OnDestroy {
  @Input() name!: string;
  @Input() description!: string;
  @Input() totalTime?: string;
  @Input() steps: Array<{ name: string; text: string; image?: string }> = [];

  private scriptElement: HTMLScriptElement | null = null;

  constructor(
    @Inject(DOCUMENT) private document: Document,
    @Inject(PLATFORM_ID) private platformId: object
  ) {}

  ngOnChanges(): void {
    this.updateStructuredData();
  }

  ngOnDestroy(): void {
    this.removeStructuredData();
  }

  private updateStructuredData(): void {
    if (!isPlatformBrowser(this.platformId) || !this.name || !this.steps.length) {
      return;
    }

    this.removeStructuredData();

    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'HowTo',
      name: this.name,
      description: this.description,
      totalTime: this.totalTime,
      step: this.steps.map((step, index) => ({
        '@type': 'HowToStep',
        position: index + 1,
        name: step.name,
        text: step.text,
        image: step.image
      }))
    };

    this.scriptElement = this.document.createElement('script');
    this.scriptElement.type = 'application/ld+json';
    this.scriptElement.id = 'structured-data-howto';
    this.scriptElement.text = JSON.stringify(structuredData);
    this.document.head.appendChild(this.scriptElement);
  }

  private removeStructuredData(): void {
    if (this.scriptElement) {
      this.scriptElement.remove();
      this.scriptElement = null;
    }
  }
}

/**
 * Pre-built Breadcrumb structured data component
 */
@Component({
  selector: 'app-breadcrumb-structured-data',
  standalone: true,
  template: ''
})
export class BreadcrumbStructuredDataComponent implements OnChanges, OnDestroy {
  @Input() items: Array<{ name: string; url: string }> = [];

  private scriptElement: HTMLScriptElement | null = null;
  private readonly baseUrl = 'https://devmer.io';

  constructor(
    @Inject(DOCUMENT) private document: Document,
    @Inject(PLATFORM_ID) private platformId: object
  ) {}

  ngOnChanges(): void {
    this.updateStructuredData();
  }

  ngOnDestroy(): void {
    this.removeStructuredData();
  }

  private updateStructuredData(): void {
    if (!isPlatformBrowser(this.platformId) || !this.items.length) {
      return;
    }

    this.removeStructuredData();

    const structuredData = {
      '@context': 'https://schema.org',
      '@type': 'BreadcrumbList',
      itemListElement: this.items.map((item, index) => ({
        '@type': 'ListItem',
        position: index + 1,
        name: item.name,
        item: item.url.startsWith('http') ? item.url : `${this.baseUrl}${item.url}`
      }))
    };

    this.scriptElement = this.document.createElement('script');
    this.scriptElement.type = 'application/ld+json';
    this.scriptElement.id = 'structured-data-breadcrumb';
    this.scriptElement.text = JSON.stringify(structuredData);
    this.document.head.appendChild(this.scriptElement);
  }

  private removeStructuredData(): void {
    if (this.scriptElement) {
      this.scriptElement.remove();
      this.scriptElement = null;
    }
  }
}
