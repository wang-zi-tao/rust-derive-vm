local a, i, n, s = {}, 0, 100000000 / 20, 1
while i < n do
  i = i + s
  a = {
    n0 = a,
    n1 = a,
    n2 = a,
    n3 = a,
    n4 = a,
    n5 = a,
    n6 = a,
    n7 = a,
    n8 = a,
    n9 = a,
    n10 = a,
    n11 = a,
    n12 = a,
    n13 = a,
    n14 = a,
    n15 = a,
    n16 = a,
    n17 = a,
    n18 = a,
    n19 = a,
  }
end
-- clua: 32*n+34MiB
-- vm_lua: 8*n+67MiB
