import { type ReactNode, useEffect, useRef, useState } from 'react';

interface MarqueeTextProps {
  text?: string;
  children?: ReactNode;
  className?: string;
}

/**
 * Text component that scrolls horizontally on hover if content is truncated.
 * Creates a continuous right-to-left loop animation.
 * Supports both text strings and React children for styled content.
 */
export function MarqueeText({ text, children, className = '' }: MarqueeTextProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLDivElement>(null);
  const [isTruncated, setIsTruncated] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const [animationDuration, setAnimationDuration] = useState(10);

  const content = children ?? text;

  useEffect(() => {
    const container = containerRef.current;
    const measure = measureRef.current;
    if (container && measure) {
      // Check if text is truncated by comparing full width to container width
      const truncated = measure.scrollWidth > container.clientWidth;
      setIsTruncated(truncated);

      // Calculate animation duration based on text width (roughly 50px per second)
      if (truncated) {
        const duration = Math.max(3, measure.scrollWidth / 50);
        setAnimationDuration(duration);
      }
    }
  }, [content]);

  return (
    <div
      ref={containerRef}
      className={`relative overflow-hidden ${className}`}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Hidden measurement element to detect truncation */}
      <div
        ref={measureRef}
        className="absolute whitespace-nowrap opacity-0 pointer-events-none"
        aria-hidden="true"
      >
        {content}
      </div>

      {!isHovered || !isTruncated ? (
        // Static truncated text when not hovered
        <div className="truncate whitespace-nowrap">{content}</div>
      ) : (
        // Animated marquee when hovered
        <div className="flex whitespace-nowrap">
          <div
            className="inline-block animate-marquee"
            style={{
              animationDuration: `${animationDuration}s`,
            }}
          >
            {content}
            {/* Spacer between loops */}
            <span className="inline-block px-4" />
            {/* Duplicate text for seamless loop */}
            {content}
          </div>
        </div>
      )}
    </div>
  );
}
