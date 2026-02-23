/**
 * Structured logging utility with level filtering and context.
 */

/** Severity levels for log messages. */
export enum LogLevel {
  Debug = 0,
  Info = 1,
  Warn = 2,
  Error = 3,
}

/** A single structured log entry. */
interface LogEntry {
  timestamp: string;
  level: string;
  logger: string;
  message: string;
  context?: Record<string, unknown>;
}

/** Maps LogLevel enum values to human-readable names. */
const LEVEL_NAMES: Record<LogLevel, string> = {
  [LogLevel.Debug]: "DEBUG",
  [LogLevel.Info]: "INFO",
  [LogLevel.Warn]: "WARN",
  [LogLevel.Error]: "ERROR",
};

/**
 * A logger instance that writes structured JSON log entries to stdout.
 */
export class Logger {
  private readonly name: string;
  private level: LogLevel;

  constructor(name: string, level: LogLevel = LogLevel.Info) {
    this.name = name;
    this.level = level;
  }

  /** Update the minimum log level. */
  setLevel(level: LogLevel): void {
    this.level = level;
  }

  /** Get the current minimum log level. */
  getLevel(): LogLevel {
    return this.level;
  }

  /** Log a debug message. */
  debug(message: string, context?: Record<string, unknown>): void {
    this.log(LogLevel.Debug, message, context);
  }

  /** Log an informational message. */
  info(message: string, context?: Record<string, unknown>): void {
    this.log(LogLevel.Info, message, context);
  }

  /** Log a warning message. */
  warn(message: string, context?: Record<string, unknown>): void {
    this.log(LogLevel.Warn, message, context);
  }

  /** Log an error message. */
  error(message: string, context?: Record<string, unknown>): void {
    this.log(LogLevel.Error, message, context);
  }

  /** Create a child logger that inherits the parent's level. */
  child(name: string): Logger {
    return new Logger(`${this.name}.${name}`, this.level);
  }

  /** Internal: format and write a log entry if it meets the level threshold. */
  private log(
    level: LogLevel,
    message: string,
    context?: Record<string, unknown>,
  ): void {
    if (level < this.level) {
      return;
    }

    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level: LEVEL_NAMES[level],
      logger: this.name,
      message,
    };

    if (context && Object.keys(context).length > 0) {
      entry.context = context;
    }

    const output = JSON.stringify(entry);
    if (level >= LogLevel.Error) {
      console.error(output);
    } else {
      console.log(output);
    }
  }
}

/**
 * Create a new named logger with the given minimum level.
 *
 * @param name - Identifier for the logger (e.g. module name).
 * @param level - Minimum severity to output.
 */
export function createLogger(
  name: string,
  level: LogLevel = LogLevel.Info,
): Logger {
  return new Logger(name, level);
}
