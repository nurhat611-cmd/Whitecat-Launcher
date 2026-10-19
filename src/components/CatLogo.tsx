export function CatLogo({ size = 34 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" fill="none" aria-hidden>
      <path d="M5 5 L28 20 L14 31 Z" fill="#f2f5fa" />
      <path d="M59 5 L36 20 L50 31 Z" fill="#f2f5fa" />
      <path d="M9.8 11.2 L22.5 19.4 L14.8 25.5 Z" fill="#3ea6ff" />
      <path d="M54.2 11.2 L41.5 19.4 L49.2 25.5 Z" fill="#3ea6ff" />
      <circle cx="32" cy="38.5" r="20.5" fill="#f2f5fa" />
      <circle cx="24.5" cy="36.5" r="3.6" fill="#2f7cf6" />
      <circle cx="39.5" cy="36.5" r="3.6" fill="#2f7cf6" />
      <circle cx="25.6" cy="35.4" r="1.2" fill="#dff2ff" />
      <circle cx="40.6" cy="35.4" r="1.2" fill="#dff2ff" />
      <circle cx="32" cy="42.2" r="1.8" fill="#3ea6ff" />
      <path
        d="M27.6 45.5 Q29.8 48.3 32 45.5 Q34.2 48.3 36.4 45.5"
        stroke="#8b96a8"
        strokeWidth="1.7"
        strokeLinecap="round"
        fill="none"
      />
    </svg>
  );
}
