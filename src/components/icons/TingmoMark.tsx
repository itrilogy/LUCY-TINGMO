import React from "react";

/** Product mark: stream + sound arcs + origin spark (LUXI palette tokens). */
const TingmoMark = ({
  width,
  height,
  size,
  className,
}: {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
}) => {
  const w = width ?? size ?? 24;
  const h = height ?? size ?? width ?? size ?? 24;

  return (
    <svg
      width={w}
      height={h}
      viewBox="0 0 48 48"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <rect width="48" height="48" rx="12" className="fill-logo-primary" />
      {/* Sound arcs */}
      <path
        d="M14 16.5c4.2-4.8 15.8-4.8 20 0"
        stroke="white"
        strokeWidth="2.4"
        strokeLinecap="round"
        opacity="0.95"
      />
      <path
        d="M16.5 20.5c3.2-3.4 11.8-3.4 15 0"
        stroke="white"
        strokeWidth="2.2"
        strokeLinecap="round"
        opacity="0.9"
      />
      <path
        d="M19.5 24c2.2-2.2 6.8-2.2 9 0"
        stroke="white"
        strokeWidth="2"
        strokeLinecap="round"
        opacity="0.85"
      />
      {/* Stream */}
      <path
        d="M26 15c3 5-5 8-1.5 13s7 6.5 4 13"
        stroke="white"
        strokeWidth="3.2"
        strokeLinecap="round"
        fill="none"
        opacity="0.95"
      />
      {/* Origin spark */}
      <circle cx="15.5" cy="31" r="3.2" fill="#00D2FF" />
      <circle cx="15.5" cy="31" r="1.5" fill="white" opacity="0.9" />
    </svg>
  );
};

export default TingmoMark;
