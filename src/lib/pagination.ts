export interface PaginatedItems<T> {
  items: T[];
  currentPage: number;
  totalPages: number;
  totalItems: number;
  startItem: number;
  endItem: number;
}

export function clampPage(page: number, totalPages: number) {
  const safeTotal = Math.max(1, totalPages);
  if (!Number.isFinite(page)) return 1;
  return Math.min(Math.max(Math.trunc(page), 1), safeTotal);
}

export function paginateItems<T>(
  items: T[],
  currentPage: number,
  pageSize: number,
): PaginatedItems<T> {
  const safePageSize = Math.max(1, Math.trunc(pageSize));
  const totalItems = items.length;
  const totalPages = Math.max(1, Math.ceil(totalItems / safePageSize));
  const page = clampPage(currentPage, totalPages);
  const startIndex = (page - 1) * safePageSize;
  const pageItems = items.slice(startIndex, startIndex + safePageSize);

  return {
    items: pageItems,
    currentPage: page,
    totalPages,
    totalItems,
    startItem: totalItems === 0 ? 0 : startIndex + 1,
    endItem: Math.min(startIndex + pageItems.length, totalItems),
  };
}
