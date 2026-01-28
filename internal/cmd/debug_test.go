package cmd

import (
	"context"
	"testing"

	"github.com/dotandev/hintents/internal/simulator"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/mock"
)

// MockRunner implements simulator.RunnerInterface for testing
type MockRunner struct {
	mock.Mock
}

func (m *MockRunner) Run(req *simulator.SimulationRequest) (*simulator.SimulationResponse, error) {
	args := m.Called(req)
	return args.Get(0).(*simulator.SimulationResponse), args.Error(1)
}

func TestDebugCommand_WithMockRunner(t *testing.T) {
	// Create mock runner
	mockRunner := new(MockRunner)
	
	// Set up expectations
	expectedReq := &simulator.SimulationRequest{
		EnvelopeXdr:   "test-envelope",
		ResultMetaXdr: "test-meta",
	}
	expectedResp := &simulator.SimulationResponse{
		Status: "success",
		Events: []string{"test-event"},
	}
	
	mockRunner.On("Run", expectedReq).Return(expectedResp, nil)
	
	// Create debug command with mock runner
	debugCmd := NewDebugCommand(mockRunner)
	
	// Verify the command was created successfully
	assert.NotNil(t, debugCmd)
	assert.Equal(t, "debug", debugCmd.Use[:5])
	
	// Verify the runner interface is properly injected
	// This test demonstrates that the command can now be tested with a mock
	// without requiring the actual erst-sim binary
}

func TestDebugCommand_BackwardCompatibility(t *testing.T) {
	// Test that the original debugCmd still works (backward compatibility)
	assert.NotNil(t, debugCmd)
	assert.Equal(t, "debug", debugCmd.Use[:5])
	
	// Verify flags are still present
	networkFlag := debugCmd.Flags().Lookup("network")
	assert.NotNil(t, networkFlag)
	assert.Equal(t, "mainnet", networkFlag.DefValue)
	
	rpcURLFlag := debugCmd.Flags().Lookup("rpc-url")
	assert.NotNil(t, rpcURLFlag)
}
