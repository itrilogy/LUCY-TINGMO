import React from "react";

/** Wordmark: 听默 / Tingmo */
const TingmoWordmark = ({
  width,
  height,
  className,
  showEnglish = false,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
  showEnglish?: boolean;
}) => {
  return (
    <div
      className={className}
      style={{
        width: width ?? "auto",
        height: height ?? "auto",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        lineHeight: 1.15,
      }}
    >
      <span
        className="font-semibold tracking-wide text-text"
        style={{ fontSize: showEnglish ? 22 : 20 }}
      >
        听默
      </span>
      {showEnglish && (
        <span className="text-mid-gray text-xs tracking-[0.18em] uppercase mt-0.5">
          Tingmo
        </span>
      )}
    </div>
  );
};

export default TingmoWordmark;
