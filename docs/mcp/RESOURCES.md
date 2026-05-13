# MCP Resources Reference -- syslog-mcp

## Overview

MCP resources expose read-only data via URI-based access. Unlike tools, resources do not perform mutations -- they return the current state of a data source.

## Available resources

syslog-mcp exposes one MCP resource:

| URI | Description | MIME type |
| --- | --- | --- |
| `syslog://schema/mcp-tool` | JSON schema for the `syslog` MCP tool and action-based parameters | `application/json` |

All log data access is through the single `syslog` MCP tool and its 8 read-only actions. Tools are preferred over log resources because queries benefit from parameterized filtering (hostname, severity, time range, FTS5 query) that URI templating cannot express efficiently.

## Future considerations

If log data resources are added in the future, they would use the `syslog://`
URI scheme:

```
syslog://stats           # Database statistics
syslog://hosts           # Host registry
syslog://hosts/{name}    # Logs for a specific host
```

## See also

- [TOOLS.md](TOOLS.md) -- MCP tool reference
