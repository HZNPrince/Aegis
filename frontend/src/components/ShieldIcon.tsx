export function ShieldIcon({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none">
      <path
        d="M12 2L4 5.5V11c0 4.55 3.4 8.82 8 9.93C16.6 19.82 20 15.55 20 11V5.5L12 2Z"
        fill="#D97757"
        opacity="0.9"
      />
      <path
        d="M9 12l2 2 4-4"
        stroke="#1F1E1D"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
