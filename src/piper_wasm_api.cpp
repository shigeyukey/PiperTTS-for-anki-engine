#include "piper.h"
#include "piper_impl.hpp"

#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#ifdef __EMSCRIPTEN__
#include <emscripten/emscripten.h>
#define PIPER_WASM_EXPORT EMSCRIPTEN_KEEPALIVE
#else
#define PIPER_WASM_EXPORT
#endif

namespace {

constexpr const char *CONFIG_PATH = "/piper_wasm_config.json";
constexpr const char *MODEL_PATH = "/piper_wasm_model.onnx";

std::vector<int32_t> g_id_buffer;

inline piper_synthesizer *as_synth(void *handle) {
  return static_cast<piper_synthesizer *>(handle);
}

} // namespace

extern "C" {

PIPER_WASM_EXPORT void *piper_wasm_create(const char *config_json,
                                          const char *espeak_data_dir) {
  if (config_json == nullptr) {
    return nullptr;
  }

  FILE *config_file = std::fopen(CONFIG_PATH, "wb");
  if (config_file == nullptr) {
    return nullptr;
  }
  std::fwrite(config_json, 1, std::strlen(config_json), config_file);
  std::fclose(config_file);

  piper_create_options options;
  piper_init_create_options(&options);
  options.model_path = MODEL_PATH;
  options.config_path = CONFIG_PATH;
  options.espeak_data_path = espeak_data_dir;

  return piper_create_with_options(&options);
}

PIPER_WASM_EXPORT void piper_wasm_free(void *handle) {
  if (handle != nullptr) {
    piper_free(as_synth(handle));
  }
}

PIPER_WASM_EXPORT int piper_wasm_start(void *handle, const char *text,
                                       float length_scale, float noise_scale,
                                       float noise_w_scale) {
  piper_synthesizer *synth = as_synth(handle);
  if ((synth == nullptr) || (text == nullptr)) {
    return PIPER_ERR_GENERIC;
  }

  piper_synthesize_options options = piper_default_synthesize_options(synth);
  options.speaker_id = 0;
  if (length_scale > 0.0F) {
    options.length_scale = length_scale;
  }
  if (noise_scale > 0.0F) {
    options.noise_scale = noise_scale;
  }
  if (noise_w_scale > 0.0F) {
    options.noise_w_scale = noise_w_scale;
  }

  return piper_synthesize_start(synth, text, &options);
}

PIPER_WASM_EXPORT int piper_wasm_next_count(void *handle) {
  piper_synthesizer *synth = as_synth(handle);
  if ((synth == nullptr) || synth->phoneme_id_queue.empty()) {
    return 0;
  }
  return static_cast<int>(synth->phoneme_id_queue.front().second.size());
}

PIPER_WASM_EXPORT int piper_wasm_next_copy(void *handle, int32_t *out,
                                           int capacity) {
  piper_synthesizer *synth = as_synth(handle);
  if ((synth == nullptr) || (out == nullptr) || (capacity <= 0)) {
    return 0;
  }
  if (synth->phoneme_id_queue.empty()) {
    return 0;
  }

  std::vector<PhonemeId> ids = std::move(synth->phoneme_id_queue.front().second);
  synth->phoneme_id_queue.pop();

  int count = static_cast<int>(ids.size());
  if (count > capacity) {
    count = capacity;
  }
  for (int i = 0; i < count; i++) {
    out[i] = static_cast<int32_t>(ids[static_cast<std::size_t>(i)]);
  }
  return count;
}

PIPER_WASM_EXPORT int piper_wasm_sample_rate(void *handle) {
  piper_synthesizer *synth = as_synth(handle);
  return (synth == nullptr) ? 0 : synth->sample_rate;
}

PIPER_WASM_EXPORT int piper_wasm_num_speakers(void *handle) {
  piper_synthesizer *synth = as_synth(handle);
  return (synth == nullptr) ? 0 : synth->num_speakers;
}

PIPER_WASM_EXPORT const char *piper_wasm_version(void) { return piper_version(); }

}
