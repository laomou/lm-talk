const AVATAR_COLORS = [
  '#12b76a', // green
  '#4c8df6', // blue
  '#f59e0b', // amber
  '#8b5cf6', // violet
  '#ec4899', // pink
  '#14b8a6', // teal
  '#f97316', // orange
  '#0ea5e9', // sky
]

export function avatarColor(seed: string | undefined | null): string {
  const s = seed || '?'
  let hash = 0
  for (let i = 0; i < s.length; i++) hash = (hash * 31 + s.charCodeAt(i)) | 0
  return AVATAR_COLORS[Math.abs(hash) % AVATAR_COLORS.length]
}
