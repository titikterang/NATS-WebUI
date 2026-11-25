// Minimal NATS producer/consumer (nats.js v3).
//   MODE=pub|sub  NATS_URL  NATS_USER  NATS_PASSWORD  NATS_CA_FILE  SUBJECT  COUNT  TIMEOUT_SECONDS
import { connect, headers } from '@nats-io/transport-node'

const env = (k, d) => process.env[k] || d
const mode = env('MODE', 'sub')
const subject = env('SUBJECT', 'demo.hello')
const count = Number(env('COUNT', '5'))
const timeoutMs = Number(env('TIMEOUT_SECONDS', '30')) * 1000

const nc = await connect({
  servers: env('NATS_URL', 'nats://127.0.0.1:4222').split(','),
  user: process.env.NATS_USER,
  pass: process.env.NATS_PASSWORD,
  tls: process.env.NATS_CA_FILE ? { caFile: process.env.NATS_CA_FILE } : undefined,
  name: `example-node-${mode}`,
})
console.log(`node: connected to ${nc.getServer()} as ${process.env.NATS_USER || '(no auth)'}`)

if (mode === 'pub') {
  for (let i = 1; i <= count; i++) {
    const h = headers()
    h.set('X-Example-Client', 'node')
    nc.publish(subject, JSON.stringify({ client: 'node', n: i, at: new Date().toISOString() }), { headers: h })
  }
  await nc.flush()
  console.log(`node: published ${count} to ${subject}`)
} else {
  const sub = nc.subscribe(subject, { max: count })
  await nc.flush() // the server has the subscription once the flush round-trips
  console.log('node: ready')
  const timer = setTimeout(() => {
    console.error(`node: timeout, received ${sub.getProcessed()}/${count}`)
    process.exit(1)
  }, timeoutMs)
  for await (const m of sub) {
    console.log(`node: ${m.subject} [${m.headers?.get('X-Example-Client') ?? '-'}] ${m.string()}`)
  }
  clearTimeout(timer)
  console.log(`node: received ${count} on ${subject}`)
}
await nc.drain()
