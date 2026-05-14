interface PagePlaceholderProps {
  title: string;
  subtitle: string;
}

export default function PagePlaceholder({ title, subtitle }: PagePlaceholderProps) {
  return (
    <div className="page">
      <header className="page__header">
        <h1 className="page__title">{title}</h1>
        <p className="page__subtitle">{subtitle}</p>
      </header>
      <div className="page__placeholder">Coming in a later phase.</div>
    </div>
  );
}
