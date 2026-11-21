# lists.awk: the failing cases of slice 6 that need only constructs of the first part, by story.
# Input: scan6.tsv (slice, path, flags, first-failure detail). Output: path, key, needs, detail.
BEGIN { FS = "\t"; OFS = "\t"; depth["let-const"] = 1; depth["template"] = 1; depth["arrow"] = 2; depth["parameters"] = 3; depth["object-pattern"] = 4 }
{
  d = $4; own = (d ~ /(let|const) declaration is not supported before slice 6|arrow function is not supported before slice 6|template literal is not supported before slice 6|default parameter is not supported before slice 6|rest parameter is not supported before slice 6|destructuring pattern is not supported before slice 6|CoverInitializedName is not supported before slice 6/) && d !~ /^[A-Za-z]+\.js: /
  if (!own) { print $2, "other", "", d; next }
  if (d ~ /destructuring pattern/ && $3 !~ /OBJPAT|ARRPAT/) $3 = $3 " UNSEEN"
  if ($2 ~ /\/(dstr|destructuring)\// && $2 ~ /\/(ary|array)-|-ary-|-array-|iterator|iter-/) $3 = $3 " ARRPAT"
  if ($2 ~ /let-[a-z]+-with-newline/) { print $2, "let-const", "let-const", d; next }
  n = split($3, f, " "); delete need; block = ""
  for (i = 1; i <= n; i++) {
    if (f[i] == "LC") need["let-const"] = 1; else if (f[i] == "TPL") need["template"] = 1; else if (f[i] == "ARROW") need["arrow"] = 1
    else if (f[i] == "DEFAULT" || f[i] == "REST") need["parameters"] = 1; else if (f[i] == "OBJPAT") need["object-pattern"] = 1
    else if (f[i] != "RAW") block = block " " f[i]
  }
  if (d ~ /(let|const) declaration/) need["let-const"] = 1; if (d ~ /arrow function/) need["arrow"] = 1; if (d ~ /template literal/) need["template"] = 1
  if (d ~ /(default|rest) parameter/) need["parameters"] = 1; if (d ~ /CoverInitializedName/ || $3 ~ /OBJPAT/) need["object-pattern"] = 1
  if (block != "") { sub(/^ /, "", block); print $2, "later", block, d; next }
  k = asorti(need, ks); list = ""; deep = ""; top = 0; ties = 0
  for (i = 1; i <= k; i++) { list = list (i > 1 ? "," : "") ks[i]; if (depth[ks[i]] > top) { top = depth[ks[i]]; deep = ks[i]; ties = 0 } else if (depth[ks[i]] == top) ties = 1 }
  print $2, (ties ? "cross" : deep), list, d
}
