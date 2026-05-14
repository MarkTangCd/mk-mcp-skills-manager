import type { AgentKind, MatrixCell, MatrixRow } from '../types/domain';

interface MatrixTableProps {
  title: string;
  rows: MatrixRow[];
  agents: AgentKind[];
  filter: string;
  onSelectCell: (cell: MatrixCell, row: MatrixRow) => void;
}

export default function MatrixTable({
  title,
  rows,
  agents,
  filter,
  onSelectCell,
}: MatrixTableProps) {
  const normalizedFilter = filter.trim().toLowerCase();
  const visibleRows = normalizedFilter
    ? rows.filter((row) => row.name.toLowerCase().includes(normalizedFilter))
    : rows;

  return (
    <section className="matrix-section">
      <div className="matrix-section__header">
        <h2 className="dashboard__section-title">{title}</h2>
        <span className="matrix-section__count">{visibleRows.length}</span>
      </div>
      {visibleRows.length === 0 ? (
        <div className="page__placeholder">No indexed resources.</div>
      ) : (
        <div className="matrix-table__scroll">
          <table className="projects__table matrix-table">
            <thead>
              <tr>
                <th>Resource</th>
                {agents.map((agent) => (
                  <th key={agent}>{agent}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {visibleRows.map((row) => (
                <tr key={row.key}>
                  <td className="matrix-table__name">{row.name}</td>
                  {agents.map((agent) => {
                    const cell = row.cells.find((item) => item.agentKind === agent);
                    return (
                      <td key={agent}>
                        {cell && (
                          <button
                            type="button"
                            className={`matrix-cell matrix-cell--${cell.status}`}
                            onClick={() => onSelectCell(cell, row)}
                            title="Show source details"
                          >
                            {cell.status}
                          </button>
                        )}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
