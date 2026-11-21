// sec-try-statement-runtime-semantics-evaluation: normal "ok"
// A throw out of the try, catch or finally block has no completion value: eval throws, or the
// catch block that takes it gives UpdateEmpty(C, undefined); a finally block around it that
// completes abruptly replaces the throw and gives UpdateEmpty(F, undefined).
var bad = [];
function check(source, expected) {
  var found;
  try { found = eval(source); } catch (e) { found = "throw " + e; }
  if (found !== expected) bad.push(source + " => " + found);
}
check("99; do { -99; try { 42; throw 1 } finally { 7 } } while (false);", "throw 1");
check("99; do { -99; try { 39 } finally { 42; throw 1 } } while (false);", "throw 1");
check("99; do { -99; try { throw 0 } catch (e) { 42; throw 1 } } while (false);", "throw 1");
check("99; do { -99; try { try { 42; throw 1 } finally { 7 } } catch (e) { } } while (false);", undefined);
check("99; do { -99; try { try { 39 } finally { 42; throw 1 } } catch (e) { } } while (false);", undefined);
check("99; do { -99; try { try { 39 } finally { 42; throw 1 } } catch (e) { e } } while (false);", 1);
check("99; do { -99; try { try { 39; break; } finally { 42; throw 1 } } catch (e) { } } while (false);", undefined);
check("99; do { -99; try { try { throw 0 } catch (e) { 42; throw 1 } } catch (e) { } } while (false);", undefined);
check("99; do { -99; try { try { 39 } finally { 42; throw 1 } } finally { break; } } while (false);", undefined);
check("99; do { -99; try { try { 39 } finally { 42; throw 1 } } finally { 43; break; } } while (false);", 43);
check("99; do { -99; try { try { 39 } catch (e) { -1 } finally { 42; throw 1 } } finally { continue; } } while (false);", undefined);
check("99; L: { -99; try { 42; throw 1 } finally { break L; } }", undefined);
bad.length === 0 ? "ok" : bad.join(" | ");
