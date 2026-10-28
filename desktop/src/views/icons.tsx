const PATHS: Record<string, string> = {
  search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4.2-4.2',
  book: 'M4 5.5A2.5 2.5 0 0 1 6.5 3H19v16H6.5A2.5 2.5 0 0 0 4 21.5V5.5zM19 19v2H6.5',
  slider: 'M4 7h9M17 7h3M4 17h4M12 17h8M13 4v6M8 14v6',
  clock: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7v5l3 2',
  pin: 'M9 4h6l-1 6 4 4H6l4-4-1-6zM12 14v6',
  close: 'M6 6l12 12M18 6L6 18',
  plus: 'M12 5v14M5 12h14',
  trash: 'M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v5M14 11v5',
  refresh: 'M20 12a8 8 0 1 1-2.3-5.7M20 4v4h-4',
  speaker: 'M11 5 6 9H3v6h3l5 4V5zM16 9a4 4 0 0 1 0 6'
}

interface Props {
  name: keyof typeof PATHS | string
  size?: number
}

export function Icon({ name, size = 16 }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d={PATHS[name] ?? ''} />
    </svg>
  )
}
