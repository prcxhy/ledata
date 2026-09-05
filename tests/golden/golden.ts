/**
 * 三方对拍（TS 侧，GUI 口径基准）：
 * 读取 Rust/Python 产出的 golden_input.json（GUI JSON 契约），
 * 用真实前端模块 formatPerformance/formatSpectra 产出期望 TSV。
 * 用法: pnpm dlx tsx tests/golden/golden.ts <input.json> <out_perf.tsv> <out_spc.tsv> <uIndex>
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { DeviceData, formatPerformance, formatSpectra } from '../../src/scripts/DeviceData';

const [, , inputPath, outPerf, outSpc, uIndexArg] = process.argv;
if (!inputPath || !outPerf || !outSpc || uIndexArg === undefined) {
    console.error('usage: tsx golden.ts <input.json> <out_perf.tsv> <out_spc.tsv> <uIndex>');
    process.exit(1);
}

const payload = JSON.parse(readFileSync(inputPath, 'utf-8'));
const devices: (DeviceData | null)[] = (payload.devices as any[]).map(d => d === null ? null :
    new DeviceData(d.name, d.is_vis, d.u, d.j, d.luminance, d.eqe, d.wavelength, d.spectra,
        d.max_j, d.max_lumi, d.max_eqe, d.valid_eqe, d.leak_j));
const nonNull = devices.filter((d): d is DeviceData => d !== null);
if (nonNull.length === 0) {
    console.error('no non-null devices in golden input');
    process.exit(1);
}

let longest = 0;
nonNull.forEach((d, i) => {
    if(d.u.length > nonNull[longest]!.u.length) longest = i;
});

writeFileSync(outPerf, formatPerformance(nonNull, longest));
writeFileSync(outSpc, formatSpectra(nonNull, parseInt(uIndexArg, 10)));
console.log('TS golden outputs written:', outPerf, outSpc);
