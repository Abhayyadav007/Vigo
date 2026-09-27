-- First accept wins.
-- KEYS: 1 offered set (current wave), 2 winner key
-- ARGV: 1 rider id, 2 winner ttl (s)
-- Returns {1, other offered riders...} won,
--         {0} not offered to this rider (or the wave expired),
--         {-1} someone else already won.
if redis.call('EXISTS', KEYS[2]) == 1 then
  return {-1}
end
if redis.call('SISMEMBER', KEYS[1], ARGV[1]) == 0 then
  return {0}
end
redis.call('SET', KEYS[2], ARGV[1], 'EX', tonumber(ARGV[2]))
redis.call('SREM', KEYS[1], ARGV[1])
local others = redis.call('SMEMBERS', KEYS[1])
redis.call('DEL', KEYS[1])
local out = {1}
for i = 1, #others do
  out[#out + 1] = others[i]
end
return out
