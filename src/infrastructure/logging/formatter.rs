use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use std::fmt;

/// Custom formatter that wraps each field in brackets for better readability
/// Format: [TIMESTAMP] [LEVEL] [FUNCTION_NAME] [TARGET: FILE:LINE]: MESSAGE
pub struct BracketedFormatter;

impl<S, N> FormatEvent<S, N> for BracketedFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Get metadata
        let metadata = event.metadata();
        
        // Write timestamp in brackets
        let now = chrono::Local::now();
        write!(writer, "[{}]  ", now.format("%Y-%m-%dT%H:%M:%S%.6fZ"))?;
        
        // Write level in brackets
        write!(writer, "[{:5}] ", metadata.level())?;
        
        // Write function name in brackets (get from span context or target)
        let function_name = if let Some(scope) = ctx.event_scope() {
            // Try to get the innermost span name (function name)
            scope
                .from_root()
                .last()
                .map(|span| span.name())
                .unwrap_or("unknown")
        } else {
            // Fall back to extracting function from target
            metadata.target()
                .rsplit("::")
                .next()
                .unwrap_or("unknown")
        };
        write!(writer, "[{}] ", function_name)?;
        
        // Write target and location in brackets
        if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
            write!(writer, "[{}: {}:{}]: ", metadata.target(), file, line)?;
        } else {
            write!(writer, "[{}]: ", metadata.target())?;
        }
        
        // Write the message
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        
        writeln!(writer)
    }
}
