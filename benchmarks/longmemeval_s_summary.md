# LongMemEval `s_cleaned` Benchmark Summary

Dataset: `data/longmemeval/longmemeval_s_cleaned.json`

## Overall

| Mode | Granularity | Time (s) | Recall@1 | Recall@5 | Recall@10 | NDCG@10 |
|------|-------------|----------|----------|----------|-----------|---------|
| raw | session | 414.8 | 0.806 | 0.966 | 0.982 | 0.889 |
| aaak | session | 502.1 | 0.830 | 0.952 | 0.974 | 0.892 |
| rooms | session | 421.5 | 0.734 | 0.878 | 0.896 | 0.808 |
| raw | turn | 2565.0 | 0.836 | 0.946 | 0.964 | 0.874 |

## Published Comparison

This is the honest apples-to-apples slice we can currently compare with the public `mempalace` README: retrieval-only `LongMemEval s_cleaned`, reported as `R@5`.

| System | Mode | LongMemEval R@5 | External API Calls | Basis |
|--------|------|-----------------|--------------------|-------|
| `mempal` | raw + session | **96.6%** | Zero | Local run in this repository |
| `mempal` | aaak + session | **95.2%** | Zero | Local run in this repository |
| `mempal` | rooms + session | **87.8%** | Zero | Local run in this repository |
| `mempalace` | Raw | **96.6%** | Zero | Public README |
| `mempalace` | AAAK | **84.2%** | Zero | Public README |

Not included in this comparison:

- `mempalace` hybrid + rerank `100%` result
- held-out numbers
- LoCoMo
- end-to-end answer-generation accuracy

Those are different benchmark paths or different evaluation layers and should not be folded into the same table without separate reproduction.

## Recommendation

- Best default retrieval mode: `raw + session`
- `aaak + session` improves top-1 and NDCG slightly, but loses recall at deeper `k` and is slower
- `rooms + session` underperforms and should not be the default benchmark mode
- `raw + turn` is too expensive for routine full runs and does not justify the cost with better overall retrieval quality

## Artifacts

- local JSONL logs are generated under `benchmarks/*.jsonl`
- those generated logs are ignored from git and can be recreated with the commands in `README.md` / `docs/usage.md`
