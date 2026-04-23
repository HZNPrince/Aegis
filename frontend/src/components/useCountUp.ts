import { useEffect, useState } from 'react';

export function useCountUp(target: number, duration = 1200, delay = 100): number {
  const [val, setVal] = useState(0);
  useEffect(() => {
    let start: number | null = null;
    let raf = 0;
    const timeout = setTimeout(() => {
      const step = (ts: number) => {
        if (start === null) start = ts;
        const p = Math.min((ts - start) / duration, 1);
        const ease = 1 - Math.pow(1 - p, 3);
        setVal(Math.round(ease * target));
        if (p < 1) raf = requestAnimationFrame(step);
        else setVal(target);
      };
      raf = requestAnimationFrame(step);
    }, delay);
    return () => {
      clearTimeout(timeout);
      cancelAnimationFrame(raf);
    };
  }, [target, duration, delay]);
  return val;
}
