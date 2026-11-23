import type { ProfileDto } from './api'

export const MIN_ACTUATION = 1

export function patchProfile(p: ProfileDto, patch: Partial<ProfileDto>): ProfileDto {
  const next = { ...p, ...patch }
  const actuation = Math.max(MIN_ACTUATION, next.actuation)
  return { ...next, actuation, release_hysteresis: Math.min(next.release_hysteresis, actuation - 1) }
}
