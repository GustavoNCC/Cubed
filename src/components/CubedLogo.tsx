interface Props {
  size?: number;
  className?: string;
}

/**
 * Logo de Cubed — letra "C" cúbica con estética cyberpunk/Minecraft.
 * Diseño: bloque isométrico formando una "C" con bordes neon.
 */
export function CubedLogo({ size = 32, className = "" }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-label="Cubed logo"
    >
      <defs>
        <linearGradient id="cubed-face-top" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#c084fc" />
          <stop offset="100%" stopColor="#a855f7" />
        </linearGradient>
        <linearGradient id="cubed-face-left" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#7c3aed" />
          <stop offset="100%" stopColor="#6d28d9" />
        </linearGradient>
        <linearGradient id="cubed-face-right" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#a21caf" />
          <stop offset="100%" stopColor="#9333ea" />
        </linearGradient>
        <filter id="cubed-glow">
          <feGaussianBlur stdDeviation="1.5" result="blur" />
          <feComposite in="SourceGraphic" in2="blur" operator="over" />
        </filter>
      </defs>

      {/* === Forma "C" construida con bloques isométricos === */}
      {/* Bloque superior */}
      <g filter="url(#cubed-glow)">
        {/* Bloque sup-izq */}
        <polygon points="8,14 24,8 40,14 24,20" fill="url(#cubed-face-top)" />
        <polygon points="8,14 8,26 24,32 24,20" fill="url(#cubed-face-left)" />
        <polygon points="24,20 24,32 40,26 40,14" fill="url(#cubed-face-right)" />

        {/* Bloque sup-der */}
        <polygon points="36,14 52,8 64,14 48,20" fill="url(#cubed-face-top)" opacity="0.85"/>
        <polygon points="48,20 48,32 64,26 64,14" fill="url(#cubed-face-right)" opacity="0.85"/>

        {/* Bloque izquierdo-medio */}
        <polygon points="0,24 16,18 32,24 16,30" fill="url(#cubed-face-top)" />
        <polygon points="0,24 0,36 16,42 16,30" fill="url(#cubed-face-left)" />
        <polygon points="16,30 16,42 32,36 32,24" fill="url(#cubed-face-right)" />

        {/* Bloque inf-izq */}
        <polygon points="8,40 24,34 40,40 24,46" fill="url(#cubed-face-top)" />
        <polygon points="8,40 8,52 24,58 24,46" fill="url(#cubed-face-left)" />
        <polygon points="24,46 24,58 40,52 40,40" fill="url(#cubed-face-right)" />

        {/* Bloque inf-der */}
        <polygon points="36,40 52,34 64,40 48,46" fill="url(#cubed-face-top)" opacity="0.85"/>
        <polygon points="48,46 48,58 64,52 64,40" fill="url(#cubed-face-right)" opacity="0.85"/>
      </g>

      {/* Neon edge highlight — top line */}
      <polyline
        points="8,14 24,8 52,8 64,14"
        stroke="#f0abfc"
        strokeWidth="1"
        strokeLinecap="round"
        opacity="0.7"
      />
      {/* Neon edge highlight — left side */}
      <polyline
        points="0,24 0,36 8,40 8,52 24,58"
        stroke="#c084fc"
        strokeWidth="1"
        strokeLinecap="round"
        opacity="0.7"
      />
      {/* Neon edge highlight — bottom line */}
      <polyline
        points="24,58 52,46 64,40"
        stroke="#f0abfc"
        strokeWidth="1"
        strokeLinecap="round"
        opacity="0.7"
      />
    </svg>
  );
}
