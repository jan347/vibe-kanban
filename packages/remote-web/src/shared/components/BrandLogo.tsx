interface BrandLogoProps {
  className?: string;
  alt?: string;
}

export function BrandLogo({
  className = "h-8 w-auto",
  alt = "GenCap Control Room",
}: BrandLogoProps) {
  return (
    <picture>
      <source
        srcSet="/gencap-logo-dark.svg"
        media="(prefers-color-scheme: dark)"
      />
      <img src="/gencap-logo.svg" alt={alt} className={className} />
    </picture>
  );
}
