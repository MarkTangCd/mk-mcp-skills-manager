import { describe, expect, it } from 'vitest';

import { clampPage, paginateItems } from './pagination';

describe('pagination helpers', () => {
  it('returns only the requested page items and metadata', () => {
    const items = Array.from({ length: 1000 }, (_, index) => `resource-${index + 1}`);

    const page = paginateItems(items, 3, 50);

    expect(page.items).toHaveLength(50);
    expect(page.items[0]).toBe('resource-101');
    expect(page.items[49]).toBe('resource-150');
    expect(page.totalPages).toBe(20);
    expect(page.totalItems).toBe(1000);
    expect(page.startItem).toBe(101);
    expect(page.endItem).toBe(150);
  });

  it('clamps out-of-range page values', () => {
    expect(clampPage(0, 8)).toBe(1);
    expect(clampPage(99, 8)).toBe(8);
    expect(clampPage(4, 8)).toBe(4);
  });
});
