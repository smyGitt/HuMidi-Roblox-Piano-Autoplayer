interface PlaceholderPageProps {
  title: string;
}

export function PlaceholderPage({ title }: PlaceholderPageProps) {
  return (
    <div className="page-placeholder">
      <h1 className="page-placeholder__title">{title}</h1>
      <p className="page-placeholder__hint">Not built yet.</p>
    </div>
  );
}
