// sec-super-keyword-runtime-semantics-evaluation: the receiver, then the key, then the base (GetSuperBase), for a read, a write, a compound assignment, a call and a delete
({ m(k) {
  super.x;
  super[k] = 1;
  super[k] += 2;
  super.f();
  delete super[k];
} });
