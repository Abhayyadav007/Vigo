-- The order was confirmed and Postgres stock decremented: turn held units
-- into sold units. Idempotent: a missing reservation is a no-op.
-- KEYS: 1 stock hash, 2 held hash, 3 reservation hash, 4 expiry zset
-- ARGV: 1 reservation id
local lines = redis.call('HGETALL', KEYS[3])
for i = 1, #lines, 2 do
  local pid, qty = lines[i], tonumber(lines[i + 1])
  local left = redis.call('HINCRBY', KEYS[2], pid, -qty)
  if left <= 0 then
    redis.call('HDEL', KEYS[2], pid)
  end
  -- Only adjust a loaded mirror; a missing entry reloads from Postgres.
  if redis.call('HEXISTS', KEYS[1], pid) == 1 then
    redis.call('HINCRBY', KEYS[1], pid, -qty)
  end
end
redis.call('DEL', KEYS[3])
redis.call('ZREM', KEYS[4], ARGV[1])
return #lines / 2
