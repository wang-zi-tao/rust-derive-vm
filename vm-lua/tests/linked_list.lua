local a, i, n, s = {}, 0, 1000000, 1
while i < n do
  i = i + s
  a = { n = a }
end
-- clua: 32*n+34MiB
-- vm_lua: 8*n+67MiB
