
import stainless.lang.*

def f(x: Int, y: Int): Int = {
  require(x > 0 && y > 0)
  x + y
  assert(x + y > 0)
  x + y
}.ensuring(res => res > 0)

def max(x: Int, y: Int): Int = {
  if x - y <= 0 then x else y
}.ensuring(res => res <= x)

def fun2(x: BigInt): (BigInt, Option[BigInt]) = {
  (x, if (x == 0) return (x + 1, Some(x + 1)) else (x + 2, Some(x + 2)))._2
}.ensuring { (_:(BigInt,Option[BigInt])) => true}
