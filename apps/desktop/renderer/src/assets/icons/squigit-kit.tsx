/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

import type { CSSProperties } from "react";

interface SizedIconProps {
  size: number;
  className?: string;
  style?: CSSProperties;
}

interface CloseCrossIconProps extends SizedIconProps {
  strokeWidth?: number;
  ariaHidden?: boolean;
}

interface SidePanelToggleIconProps extends SizedIconProps {
  active?: boolean;
}

interface CaptureSquiggleIconProps {
  className?: string;
  pathClassName?: string;
}

interface OcrCircularArcIconProps {
  className?: string;
  trackClassName?: string;
  arcClassName?: string;
  strokeDasharray: string;
}

const OCR_CIRCULAR_PATH =
  "M18 2.0845 a 15.9155 15.9155 0 0 1 0 31.831 a 15.9155 15.9155 0 0 1 0 -31.831";

export const AppLogo = ({ size, color }: { size: number; color: string }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 827 827"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
  >
    <path
      d="M116.791 593.519C116.897 624.434 129.224 654.054 151.085 675.915C172.946 697.776 202.565 710.104 233.48 710.21H326.953V827H233.28C202.632 826.987 172.287 820.938 143.978 809.197C115.668 797.457 89.9473 780.254 68.2852 758.573C46.6231 736.893 29.4429 711.157 17.7266 682.837C6.01032 654.517 -0.0131451 624.166 0 593.519V500.047H116.791V593.519ZM662.22 603.163C678.913 596.249 697.281 594.44 715.002 597.965C732.723 601.49 749 610.191 761.776 622.967C774.553 635.743 783.254 652.021 786.779 669.742C790.304 687.463 788.494 705.831 781.58 722.523C774.666 739.216 762.957 753.484 747.934 763.522C732.91 773.561 715.248 778.919 697.18 778.919C672.951 778.919 649.714 769.293 632.582 752.161C615.45 735.029 605.825 711.793 605.825 687.564C605.825 669.496 611.183 651.834 621.221 636.811C631.259 621.787 645.527 610.077 662.22 603.163ZM326.953 116.862H233.48C202.565 116.968 172.946 129.304 151.085 151.178C129.224 173.052 116.897 202.689 116.791 233.624V326.953H0V233.424C0.0131396 202.757 6.06309 172.393 17.8037 144.065C29.5443 115.738 46.7462 90.0024 68.4268 68.3271C90.1074 46.6518 115.842 29.4608 144.162 17.7373C172.482 6.01382 202.833 -0.0131094 233.48 0H326.953V116.862ZM593.719 0C624.367 0.0131396 654.712 6.06309 683.022 17.8037C711.332 29.5443 737.053 46.7462 758.715 68.4268C780.377 90.1074 797.557 115.842 809.273 144.162C820.99 172.482 827.013 202.833 827 233.48V326.953H710.209V233.48C710.103 202.565 697.775 172.946 675.914 151.085C654.053 129.224 624.434 116.897 593.519 116.791H500.046V0H593.719Z"
      fill={color}
    />
    <path
      d="M413.877 63.4409C413.877 256.226 569.55 412.651 762.047 413.871L764.314 413.877C570.773 413.877 413.877 570.773 413.877 764.314C413.877 571.529 258.205 415.103 65.7075 413.884L63.4409 413.877C256.982 413.877 413.877 256.982 413.877 63.4409Z"
      fill={color}
    />
  </svg>
);

export const InlineChevronRight = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <path d="m9 18 6-6-6-6" />
  </svg>
);

export const InlineChevronLeft = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <path d="m15 18-6-6 6-6" />
  </svg>
);

export const TranslateIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <path d="m5 8 6 6" />
    <path d="m4 14 6-6 2-3" />
    <path d="M2 5h12" />
    <path d="M7 2h1" />
    <path d="m22 22-5-10-5 10" />
    <path d="M14 18h6" />
  </svg>
);

export const ExpandTextareaIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <path d="M4 10V4h6" />
    <path d="M20 14v6h-6" />
  </svg>
);

export const CollapseTextareaIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <path d="M8 2v6H2" />
    <path d="M16 22v-6h6" />
  </svg>
);

export const CloseCrossIcon = ({
  size,
  className,
  style,
  strokeWidth = 3,
  ariaHidden = false,
}: CloseCrossIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={strokeWidth}
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
    aria-hidden={ariaHidden}
  >
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

export const DragDotsIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    className={className}
    style={style}
  >
    <path d="M7 19c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zM7 3c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zM7 11c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zM17 19c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zM17 3c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zM17 11c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z" />
  </svg>
);

export const CircularSpinnerIcon = ({
  size,
  className,
  style,
}: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <path d="M21 12a9 9 0 1 1-6.219-8.56" />
  </svg>
);

export const CheckmarkIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <polyline points="20 6 9 17 4 12" />
  </svg>
);

export const CopyImageIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 17 17"
    fill="currentColor"
    className={className}
    style={style}
  >
    <path fillRule="evenodd" clipRule="evenodd" d="M13 0C15.2091 0 17 1.79086 17 4V9C17 11.14 15.3194 12.8879 13.2061 12.9951L13 13C13 15.2091 11.2091 17 9 17H4L3.79395 16.9951C1.68056 16.8879 0 15.14 0 13V8C0 5.79086 1.79086 4 4 4C4 1.79086 5.79086 8.0532e-08 8 0H13ZM4 6C2.89543 6 2 6.89543 2 8V13C2 14.1046 2.89543 15 4 15H9C10.1046 15 11 14.1046 11 13V8C11 6.89543 10.1046 6 9 6H4ZM8 2C6.89543 2 6 2.89543 6 4H9C11.2091 4 13 5.79086 13 8V11C14.1046 11 15 10.1046 15 9V4C15 2.89543 14.1046 2 13 2H8Z" fill="black"/>
  </svg>
);

export const SaveFileIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 17"
    fill="currentColor"
    className={className}
    style={style}
  >
    <path d="M15.1123 11.5596C15.6006 11.5596 16 11.9557 16 12.4473V14.2256C15.9998 15.6967 14.8085 16.8906 13.334 16.8906H2.66602C1.19482 16.8906 0.000233216 15.7 0 14.2256V12.4473C0 11.9558 0.402422 11.5596 0.890625 11.5596C1.37887 11.5596 1.77832 11.9557 1.77832 12.4473V14.2256C1.77855 14.7136 2.17461 15.1123 2.66602 15.1123H13.3379C13.8259 15.1122 14.2254 14.7169 14.2256 14.2256V12.4473C14.2256 11.9591 14.6209 11.5597 15.1123 11.5596ZM8 0C8.49155 0 8.8877 0.399448 8.8877 0.887695V9.85547L11.8174 6.92969C12.1638 6.58013 12.7279 6.58004 13.0742 6.92969C13.4203 7.27606 13.4204 7.84022 13.0742 8.18652L8.62695 12.6299C8.28056 12.9763 7.71651 12.9763 7.37012 12.6299L2.92578 8.18262C2.57983 7.83619 2.57954 7.27203 2.92578 6.92578C3.27203 6.57954 3.83619 6.57983 4.18262 6.92578L7.1123 9.85547V0.887695C7.1123 0.399448 7.50845 1.03261e-06 8 0Z" fill="black"/>
  </svg>
);

export const SidePanelToggleIcon = ({
  size,
  className,
  style,
  active = false,
}: SidePanelToggleIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.2"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    style={style}
  >
    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
    <path
      d="M9 3V21H5C3.89543 21 3 20.1046 3 19V5C3 3.89543 3.89543 3 5 3H9Z"
      fill={active ? "currentColor" : "none"}
      stroke={active ? "none" : "currentColor"}
    />
    <line x1="9" y1="3" x2="9" y2="21" />
  </svg>
);

export const WindowMinimizeIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 12 12"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.1"
    className={className}
    style={style}
  >
    <line x1="1" y1="6" x2="11" y2="6" />
  </svg>
);

export const WindowMaximizeIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 12 12"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.1"
    className={className}
    style={style}
  >
    <rect x="1" y="1" width="10" height="10" />
  </svg>
);

export const WindowCloseIcon = ({ size, className, style }: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 12 12"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.1"
    className={className}
    style={style}
  >
    <line x1="1" y1="1" x2="11" y2="11" />
    <line x1="11" y1="1" x2="1" y2="11" />
  </svg>
);

export const SidePanelNewThreadIcon = ({
  size,
  className,
  style,
}: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 121 118"
    fill="none"
    className={className}
    style={style}
  >
    <path
      d="M104.48 72.7959V91.8586C104.48 103.457 95.078 112.859 83.4801 112.859H25.4675C13.8696 112.859 4.46753 103.457 4.46753 91.8586V37.3047C4.46753 25.7067 13.8695 16.3047 25.4675 16.3047H47.4124"
      stroke="currentColor"
      strokeWidth="8.93484"
      strokeLinecap="round"
    />
    <path
      d="M43.1669 53.6424L81.7635 16.6424C87.9474 10.7144 97.7666 10.9223 103.695 17.1063C109.623 23.2902 109.415 33.1088 103.232 39.0369L64.7774 75.9004C61.9576 78.6034 58.3471 80.3334 54.4737 80.8374L35.8215 83.2635C34.9464 83.3772 34.2154 82.6027 34.3799 81.7356L37.8515 63.4429C38.5612 59.7029 40.4188 56.2767 43.1669 53.6424Z"
      stroke="currentColor"
      strokeWidth="8.3584"
    />
  </svg>
);

export const SidePanelSquigitsIcon = ({
  size,
  className,
  style,
}: SizedIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 138 149"
    fill="none"
    className={className}
    style={style}
  >
    <path
      d="M19.4877 37.4318L84.6124 33.8414C92.9872 33.3798 100.151 39.795 100.613 48.1698L103.748 105.039C104.21 113.414 97.7947 120.578 89.4199 121.04L24.2951 124.63C15.9203 125.092 8.75652 118.677 8.2946 110.302L5.15924 53.4324C4.69768 45.0575 11.1129 37.8937 19.4877 37.4318Z"
      stroke="currentColor"
      strokeWidth="8.20785"
    />
    <path
      d="M31.7285 25.4662L31.7285 23.3972C31.7285 11.9698 41.6062 3.04716 52.9748 4.20517L115.808 10.6054C126.407 11.685 134.125 21.1528 133.045 31.7523L126.381 97.1711C125.778 103.098 120.787 107.606 114.829 107.606"
      stroke="currentColor"
      strokeWidth="8.20785"
      strokeLinecap="round"
    />
    <path
      d="M98.5279 117.328L49.9828 74.7361C42.6208 68.2769 31.587 68.3591 24.3221 74.9275L6.88965 90.6885"
      stroke="currentColor"
      strokeWidth="8.20785"
    />
    <circle cx="73.0312" cy="59.4555" r="8.72703" fill="currentColor" />
  </svg>
);

export const CaptureCursorIcon = ({ className }: { className?: string }) => (
  <svg viewBox="0 0 255 362" fill="none" className={className}>
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M0 305.971V0L221.195 221.984H91.7908L83.9476 224.353L0 305.971Z"
      fill="var(--c-raw-036)"
    />
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M173.363 318.853L104.567 348.18L15.219 136.322L85.5601 106.651L173.363 318.853Z"
      fill="var(--c-raw-036)"
    />
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M147.915 305.849L112.725 320.636L53.5669 179.754L88.6993 164.947L147.915 305.849Z"
      fill="var(--c-raw-018)"
    />
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M19.0833 45.9883V259.738L75.7417 204.982L83.9094 202.327H174.899L19.0833 45.9883Z"
      fill="var(--c-raw-018)"
    />
  </svg>
);

export const GoogleLensIcon = ({ size, color }: { size: number; color: string }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill={color}
    xmlns="http://www.w3.org/2000/svg"
  >
    <path d="M0 0h24v24H0z" fill="none" />
    <path d="M21,9v4h-2V9c0-1.1-0.9-2-2-2H7C5.9,7,5,7.9,5,9v3H3V9c0-2.21,1.79-4,4-4h2l1-2h4l1,2h2C19.21,5,21,6.79,21,9z M12,21H7 c-2.21,0-4-1.79-4-4v-2h2v2c0,1.1,0.9,2,2,2h5V21z M18,16c1.1,0,2,0.9,2,2s-0.9,2-2,2s-2-0.9-2-2S16.9,16,18,16z M12,10   c1.66,0,3,1.34,3,3s-1.34,3-3,3s-3-1.34-3-3S10.34,10,12,10z" />
  </svg>
);

export const CaptureSquiggleIcon = ({
  className,
  pathClassName,
}: CaptureSquiggleIconProps) => (
  <svg viewBox="0 0 180 180" className={className}>
    <path
      d="M 133,33 C 100,25 67,30 50,50 C 25,75 33,117 58,142 C 83,158 133,150 150,125 C 167,100 158,50 133,33 Z"
      className={pathClassName}
    />
  </svg>
);

export const OcrCircularArcIcon = ({
  className,
  trackClassName,
  arcClassName,
  strokeDasharray,
}: OcrCircularArcIconProps) => (
  <svg viewBox="0 0 36 36" className={className}>
    <path className={trackClassName} d={OCR_CIRCULAR_PATH} />
    <path className={arcClassName} strokeDasharray={strokeDasharray} d={OCR_CIRCULAR_PATH} />
  </svg>
);
