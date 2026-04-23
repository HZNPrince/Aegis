import { healthColor } from '../utils';
import { useCountUp } from './useCountUp';

interface Props {
  score: number;
  size?: number;
  animate?: boolean;
}

export function HealthGauge({ score, size = 220, animate = true }: Props) {
  const animated = useCountUp(score);
  const displayed = animate ? animated : score;
  const color = healthColor(displayed);
  const cx = size / 2;
  const cy = size / 2;
  const R = size * 0.38;
  const strokeW = size * 0.055;
  const START_DEG = 215;
  const SWEEP = 250;

  const polar = (r: number, deg: number) => {
    const rad = ((deg - 90) * Math.PI) / 180;
    return { x: cx + r * Math.cos(rad), y: cy + r * Math.sin(rad) };
  };
  const arcPath = (start: number, end: number) => {
    const s = polar(R, start);
    const e = polar(R, end);
    const large = end - start > 180 ? 1 : 0;
    return `M ${s.x.toFixed(2)} ${s.y.toFixed(2)} A ${R} ${R} 0 ${large} 1 ${e.x.toFixed(2)} ${e.y.toFixed(2)}`;
  };
  const fillEnd = START_DEG + (displayed / 100) * SWEEP;

  const ticks = [0, 25, 50, 75, 100].map((v) => {
    const deg = START_DEG + (v / 100) * SWEEP;
    const inner = polar(R - strokeW * 0.9, deg);
    const outer = polar(R + strokeW * 0.1, deg);
    return { inner, outer, v };
  });

  return (
    <div style={{ position: 'relative', width: size, height: size }}>
      <svg width={size} height={size} style={{ overflow: 'visible' }}>
        <path
          d={arcPath(START_DEG, START_DEG + SWEEP)}
          fill="none"
          stroke="rgba(255,255,255,0.08)"
          strokeWidth={strokeW}
          strokeLinecap="round"
        />
        {displayed > 0 && (
          <path
            d={arcPath(START_DEG, fillEnd)}
            fill="none"
            stroke={color}
            strokeWidth={strokeW}
            strokeLinecap="round"
            style={{
              filter: `drop-shadow(0 0 ${strokeW * 0.6}px ${color}66)`,
              transition: 'stroke 0.6s ease',
            }}
          />
        )}
        {ticks.map(({ inner, outer, v }) => (
          <line
            key={v}
            x1={inner.x}
            y1={inner.y}
            x2={outer.x}
            y2={outer.y}
            stroke="rgba(255,255,255,0.15)"
            strokeWidth={1.5}
          />
        ))}
      </svg>
      <div
        style={{
          position: 'absolute',
          inset: 0,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          paddingTop: size * 0.08,
        }}
      >
        <span
          style={{
            fontFamily: "'Fraunces', serif",
            fontSize: size * 0.28,
            fontWeight: 600,
            color,
            lineHeight: 1,
            letterSpacing: '-0.02em',
            transition: 'color 0.6s',
          }}
        >
          {displayed}
        </span>
        <span
          style={{
            fontFamily: "'Inter', sans-serif",
            fontSize: size * 0.07,
            color: 'rgba(245,244,239,0.45)',
            letterSpacing: '0.08em',
            textTransform: 'uppercase',
            marginTop: 2,
          }}
        >
          Health
        </span>
      </div>
    </div>
  );
}
