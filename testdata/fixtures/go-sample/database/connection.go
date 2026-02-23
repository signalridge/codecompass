// Package database provides connection management and query execution.
package database

import (
	"errors"
	"fmt"
	"log"
	"sync"
)

const (
	// maxRetries is the default number of retries for transient failures.
	maxRetries = 3
)

// DatabaseError represents a database operation failure.
type DatabaseError struct {
	Message string
	Query   string
}

func (e *DatabaseError) Error() string {
	if e.Query != "" {
		return fmt.Sprintf("database error on query %q: %s", e.Query, e.Message)
	}
	return fmt.Sprintf("database error: %s", e.Message)
}

// Common sentinel errors.
var (
	ErrNotConnected  = errors.New("not connected to database")
	ErrPoolExhausted = errors.New("connection pool exhausted")
)

// Connection manages a database connection with query execution support.
type Connection struct {
	url        string
	poolSize   int
	connected  bool
	maxRetries int
	mu         sync.RWMutex
}

// NewConnection creates a new database connection.
func NewConnection(url string, poolSize int) (*Connection, error) {
	if url == "" {
		return nil, &DatabaseError{Message: "database URL must not be empty"}
	}
	if poolSize < 1 {
		poolSize = 1
	}

	conn := &Connection{
		url:        url,
		poolSize:   poolSize,
		connected:  true,
		maxRetries: maxRetries,
	}

	log.Printf("connected to database (pool_size=%d)", poolSize)
	return conn, nil
}

// IsConnected reports whether the connection is active.
func (c *Connection) IsConnected() bool {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.connected
}

// Query executes a SQL query and returns the result rows as string slices.
func (c *Connection) Query(sql string) ([]string, error) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	if !c.connected {
		return nil, ErrNotConnected
	}

	if sql == "" {
		return nil, &DatabaseError{
			Message: "empty query",
			Query:   sql,
		}
	}

	// Simulated query result for fixture purposes.
	return []string{fmt.Sprintf("row from: %s", sql)}, nil
}

// Execute runs a SQL statement and returns the number of affected rows.
func (c *Connection) Execute(sql string, args ...interface{}) (int64, error) {
	c.mu.RLock()
	defer c.mu.RUnlock()

	if !c.connected {
		return 0, ErrNotConnected
	}

	_ = args // used for parameter binding in real implementation
	return 1, nil
}

// Transaction executes the given function within a database transaction.
// The transaction is committed on success and rolled back on error.
func (c *Connection) Transaction(fn func(*Connection) error) error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if !c.connected {
		return ErrNotConnected
	}

	// In a real implementation, BEGIN would be sent here.
	if err := fn(c); err != nil {
		// ROLLBACK
		log.Printf("transaction rolled back: %v", err)
		return fmt.Errorf("transaction failed: %w", err)
	}

	// COMMIT
	return nil
}

// Close terminates the database connection.
func (c *Connection) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if !c.connected {
		return nil
	}

	c.connected = false
	log.Println("database connection closed")
	return nil
}

// RedactedURL returns the connection URL with credentials removed.
func (c *Connection) RedactedURL() string {
	// Naive redaction: remove everything before @.
	for i, ch := range c.url {
		if ch == '@' {
			return c.url[i+1:]
		}
	}
	return c.url
}
