// TODO(local-first): login is implicit in single-user local mode. This stub
// keeps consumers compiling but renders nothing meaningful.

interface LoginRequiredPromptProps {
  className?: string;
  title?: string;
  description?: string;
  actionLabel?: string;
}

export function LoginRequiredPrompt({
  className,
  title,
  description,
}: LoginRequiredPromptProps) {
  return (
    <div className={className}>
      {title ? <p className="text-sm font-medium">{title}</p> : null}
      {description ? <p className="text-xs text-low">{description}</p> : null}
    </div>
  );
}
