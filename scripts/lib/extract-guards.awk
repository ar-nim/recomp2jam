# extract-guards.awk — shared '## Guards' section parser.
#
# Reads issue text on stdin and prints the lines between a '## Guards'
# heading and the next '##' heading (inclusive-exclusive). The caller pipes
# the output through the name extractor (backticked or snake_case tokens).
#
# Single source of truth for the section extraction shared by
# scripts/tracker-audit.sh and scripts/guard-liveness.sh (issue #186).

/^##+ *Guards/ { in_section = 1; next }
/^##+ / { in_section = 0 }
in_section { print }
