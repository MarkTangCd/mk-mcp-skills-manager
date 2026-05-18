import type { PaginatedItems } from '../lib/pagination';

interface PaginationControlsProps {
  page: PaginatedItems<unknown>;
  onPageChange: (page: number) => void;
}

export default function PaginationControls({ page, onPageChange }: PaginationControlsProps) {
  if (page.totalItems === 0) return null;

  return (
    <div className="pagination" aria-label="Pagination">
      <span>
        {page.startItem}-{page.endItem} of {page.totalItems}
      </span>
      <div className="pagination__actions">
        <button
          type="button"
          onClick={() => onPageChange(page.currentPage - 1)}
          disabled={page.currentPage <= 1}
        >
          Previous
        </button>
        <span>
          Page {page.currentPage} / {page.totalPages}
        </span>
        <button
          type="button"
          onClick={() => onPageChange(page.currentPage + 1)}
          disabled={page.currentPage >= page.totalPages}
        >
          Next
        </button>
      </div>
    </div>
  );
}
