#ifndef PIPER_WASM_ORT_STUB_H_
#define PIPER_WASM_ORT_STUB_H_

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

// ---- C types ---------------------------------------------------------------
struct OrtValue;

typedef enum OrtLoggingLevel {
  ORT_LOGGING_LEVEL_VERBOSE = 0,
  ORT_LOGGING_LEVEL_INFO = 1,
  ORT_LOGGING_LEVEL_WARNING = 2,
  ORT_LOGGING_LEVEL_ERROR = 3,
  ORT_LOGGING_LEVEL_FATAL = 4,
} OrtLoggingLevel;

typedef enum OrtAllocatorType {
  OrtInvalidAllocator = -1,
  OrtDeviceAllocator = 0,
  OrtArenaAllocator = 1,
} OrtAllocatorType;

typedef enum OrtMemType {
  OrtMemTypeCPUInput = -2,
  OrtMemTypeCPUOutput = -1,
  OrtMemTypeCPU = OrtMemTypeCPUOutput,
  OrtMemTypeDefault = 0,
} OrtMemType;

typedef enum GraphOptimizationLevel {
  ORT_DISABLE_ALL = 0,
  ORT_ENABLE_BASIC = 1,
  ORT_ENABLE_EXTENDED = 2,
  ORT_ENABLE_ALL = 3,
} GraphOptimizationLevel;

typedef enum ExecutionMode {
  ORT_SEQUENTIAL = 0,
  ORT_PARALLEL = 1,
} ExecutionMode;

// ---- C++ API ---------------------------------------------------------------
namespace Ort {

class Env {
public:
  Env() = default;
  Env(OrtLoggingLevel /*log_severity_level*/, const char * /*logid*/) {}
};

class SessionOptions {
public:
  SessionOptions() = default;

  void DisableCpuMemArena() {}
  void DisableMemPattern() {}
  void DisableProfiling() {}
  void SetIntraOpNumThreads(int /*num_intra_op_threads*/) {}
  void SetInterOpNumThreads(int /*num_inter_op_threads*/) {}
  void SetGraphOptimizationLevel(GraphOptimizationLevel /*level*/) {}
  void SetExecutionMode(ExecutionMode /*execution_mode*/) {}
};

class AllocatorWithDefaultOptions {
public:
  AllocatorWithDefaultOptions() = default;
};

class MemoryInfo {
public:
  MemoryInfo() = default;

  static MemoryInfo CreateCpu(OrtAllocatorType /*allocator_type*/,
                              OrtMemType /*mem_type*/ = OrtMemTypeDefault) {
    return MemoryInfo();
  }
};

class TensorTypeAndShapeInfo {
public:
  std::vector<int64_t> GetShape() const { return std::vector<int64_t>(); }
};

class RunOptions {
public:
  explicit RunOptions(void * /*run_options*/ = nullptr) {}
};

class Value {
public:
  Value() = default;

  template <typename T>
  static Value CreateTensor(const MemoryInfo & /*memory_info*/,
                            T * /*p_data*/, size_t /*p_data_element_count*/,
                            const int64_t * /*shape*/, size_t /*shape_len*/) {
    return Value();
  }

  bool IsTensor() const { return false; }

  TensorTypeAndShapeInfo GetTensorTypeAndShapeInfo() const {
    return TensorTypeAndShapeInfo();
  }

  template <typename T> const T *GetTensorData() const { return nullptr; }

  OrtValue *release() { return nullptr; }
};

class Session {
public:
  Session() = default;

  template <typename ModelPathT>
  Session(const Env & /*env*/, const ModelPathT * /*model_path*/,
          const SessionOptions & /*options*/) {}

  std::vector<std::string> GetOutputNames() const {
    return std::vector<std::string>();
  }

  std::vector<Value> Run(const RunOptions & /*run_options*/,
                         const char *const * /*input_names*/,
                         const Value * /*input_values*/, size_t /*input_count*/,
                         const char *const * /*output_names*/,
                         size_t /*output_count*/) {
    return std::vector<Value>();
  }
};

namespace detail {
inline void OrtRelease(OrtValue * /*value*/) {}
} // namespace detail

} // namespace Ort

#endif // PIPER_WASM_ORT_STUB_H_
