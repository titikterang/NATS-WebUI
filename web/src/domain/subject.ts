// NATS subject rules. Same rules as SubjectPattern in src/domain/client.rs, so the form can
// reject bad input before the backend does.

/** Returns an error message, or null if `raw` is a valid subject or subscription pattern. */
export function subjectError(raw: string): string | null {
  const s = raw.trim()
  if (!s) return 'subject must not be empty'
  if (/\s/.test(s)) return `subject "${s}" must not contain spaces`
  const tokens = s.split('.')
  for (const [i, t] of tokens.entries()) {
    if (!t) return `subject "${s}" has an empty token`
    if (t === '>' && i !== tokens.length - 1) return `subject "${s}": '>' must be the last token`
    if (t.length > 1 && /[*>]/.test(t)) return `subject "${s}": wildcards must be whole tokens`
  }
  return null
}

export function isWildcard(subject: string): boolean {
  return subject.trim().split('.').some((t) => t === '*' || t === '>')
}

/** Error for a subject used to publish (no wildcards allowed). */
export function publishSubjectError(raw: string): string | null {
  return subjectError(raw) ?? (isWildcard(raw) ? 'cannot publish to a wildcard subject' : null)
}

/** Error for a host field: a hostname or IP, without scheme, path or spaces. */
export function hostError(raw: string): string | null {
  const h = raw.trim()
  if (!h) return 'host is required'
  if (h.includes('://') || h.includes('/') || /\s/.test(h)) return 'use a hostname or IP address, without scheme, path or spaces'
  return null
}
