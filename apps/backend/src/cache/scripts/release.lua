-- Give a reservation's units back (payment failed/expired, order cancelled
-- before confirmation). Idempotent: a missing reservation is a no-op.
-- KEYS: 1 held hash, 2 reservation hash, 3 expiry zset
-- ARGV: 1 reservation id
-- Returns the number of lines released.
local lines = redis.call('HGETALL', KEYS[2])
for i = 1, #lines, 2 do
  local left = redis.call('HINCRBY', KEYS[1], lines[i], -tonumber(lines[i + 1]))
  if left <= 0 then
    redis.call('HDEL', KEYS[1], lines[i])
  end
end
redis.call('DEL', KEYS[2])
redis.call('ZREM', KEYS[3], ARGV[1])
return #lines / 2
