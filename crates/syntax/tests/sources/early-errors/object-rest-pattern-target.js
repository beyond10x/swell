// sec-destructuring-assignment-static-semantics-early-errors: SyntaxError "the target of a rest property may not be a pattern"
({ ...{ a } } = o);
