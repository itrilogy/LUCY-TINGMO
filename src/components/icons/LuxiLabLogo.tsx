import React from "react";
import luxiLabUrl from "../../assets/luxi-lab.svg";

/** Developer / lab identity mark (鹿溪联合创新实验室) — original artwork with rounded frame. */
const LuxiLabLogo = ({
  width = 96,
  height = 96,
  className,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
}) => (
  <img
    src={luxiLabUrl}
    alt=""
    width={width}
    height={height}
    className={`rounded-2xl overflow-hidden shadow-sm ring-1 ring-border/60 object-cover bg-surface ${className ?? ""}`}
    draggable={false}
  />
);

export default LuxiLabLogo;
