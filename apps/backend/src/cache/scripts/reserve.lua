-- Reserve every cart line atomically, or nothing.
-- KEYS: 1 stock hash, 2 held hash, 3 reservation hash, 4 expiry zset
-- ARGV: 1 reservation id, 2 expires-at (unix ms), 3 key ttl (s), 4 line count,
--       then product id / quantity pairs.
-- Returns {1, ""} reserved (or already reserved: idempotent),
--         {0, product} not enough stock, {-1, product} stock mirror not loaded.
if redis.call('EXISTS', KEYS[3]) == 1 then
  return {1, ''}
end
local n = tonumber(ARGV[4])
for i = 0, n - 1 do
  local pid = ARGV[5 + 2 * i]
  local qty = tonumber(ARGV[6 + 2 * i])
  local stock = redis.call('HGET', KEYS[1], pid)
  if not stock then
    return {-1, pid}
  end
  local held = tonumber(redis.call('HGET', KEYS[2], pid) or '0')
  if tonumber(stock) - held < qty then
    return {0, pid}
  end
end
for i = 0, n - 1 do
  local pid = ARGV[5 + 2 * i]
  local qty = tonumber(ARGV[6 + 2 * i])
  redis.call('HINCRBY', KEYS[2], pid, qty)
  redis.call('HSET', KEYS[3], pid, qty)
end
redis.call('EXPIRE', KEYS[3], tonumber(ARGV[3]))
redis.call('ZADD', KEYS[4], tonumber(ARGV[2]), ARGV[1])
return {1, ''}
