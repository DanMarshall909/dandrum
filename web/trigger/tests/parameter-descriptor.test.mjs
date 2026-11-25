import test from 'node:test';
import assert from 'node:assert/strict';
import {createParameterDescriptor,createParameterVariant,fractionDragSteps,stepRational} from '../shell/parameter-descriptor.mjs';

const closeTo=(actual,expected,tolerance=1e-12)=>assert.ok(Math.abs(actual-expected)<tolerance,`${actual} should be within ${tolerance} of ${expected}`);

test('a shared parameter descriptor owns transfer steps marks formatting parsing and accessible text',()=>{
  const descriptor=createParameterDescriptor({
    kind:'physical',min:20,max:20000,defaultValue:2000,scale:'log',unit:'Hz',precision:0,
    step:100,fineStep:10,coarseStep:1000,marks:[20,100,1000,10000,20000],
    formatNative:value=>Math.round(value)+' Hz',
    parseNative:text=>{const match=String(text).trim().match(/^([+-]?(?:\d+(?:\.\d*)?|\.\d+))\s*(k?hz)?$/i);if(!match)return null;return Number(match[1])*(match[2]?.toLowerCase()==='khz'?1000:1);},
    accessibleNative:value=>Math.round(value)+' hertz',
  });
  assert.equal(descriptor.kind,'physical');assert.equal(descriptor.min,20);assert.equal(descriptor.max,20000);assert.equal(descriptor.defaultValue,2000);
  assert.equal(descriptor.step,100);assert.equal(descriptor.fineStep,10);assert.equal(descriptor.coarseStep,1000);assert.deepEqual(descriptor.marks,[20,100,1000,10000,20000]);
  assert.ok(Math.abs(descriptor.toNative(.5)-Math.sqrt(20*20000))<1e-10);
  assert.ok(Math.abs(descriptor.toPosition(2000)-Math.log(100)/Math.log(1000))<1e-12);
  assert.ok(Math.abs(descriptor.toNative(descriptor.toPosition(4321))-4321)<1e-9);
  assert.equal(descriptor.format(descriptor.toPosition(2000)),'2000 Hz');
  assert.equal(descriptor.accessibleText(descriptor.toPosition(2000)),'2000 hertz');
  const parsed=descriptor.parseEdit('2 kHz');assert.equal(parsed.valid,true);assert.equal(parsed.nativeValue,2000);assert.ok(Math.abs(parsed.position-descriptor.toPosition(2000))<1e-12);assert.equal(parsed.rational,null);
  assert.deepEqual(descriptor.parseEdit('not a frequency'),{valid:false,error:'Enter a valid Hz value'});
});

test('linear descriptors clamp and snap native values before publication',()=>{
  const descriptor=createParameterDescriptor({kind:'integer',min:1,max:16,defaultValue:8,step:1,precision:0,unit:'voices'});
  assert.equal(descriptor.defaultPosition,7/15);assert.equal(descriptor.toNative(.5),9);assert.equal(descriptor.toPosition(9),8/15);
  assert.equal(descriptor.normalizeNative(20),16);assert.equal(descriptor.normalizeNative(4.6),5);assert.equal(descriptor.normalizeNative(-2),1);
  assert.equal(descriptor.format(descriptor.toPosition(5)),'5 voices');assert.equal(descriptor.accessibleText(descriptor.toPosition(5)),'5 voices');
});

test('default descriptor parsing formatting and invalid definitions are explicit',()=>{
  const measured=createParameterDescriptor({min:0,max:10,defaultValue:2,unit:'kg',step:.1,precision:1});
  assert.equal(measured.format(.2),'2.0 kg');assert.equal(measured.parseEdit('2 kg').nativeValue,2);assert.equal(measured.parseEdit('bad').valid,false);
  const plain=createParameterDescriptor({min:0,max:1,step:0});assert.equal(plain.parseEdit('.5').nativeValue,.5);assert.equal(plain.normalizeNative(.333),.333);
  for(const options of [{min:1,max:1},{min:0,max:10,scale:'log'},{kind:'enum',values:[1,1]}])assert.throws(()=>createParameterDescriptor(options),TypeError);
});

test('percentage unit interval bipolar and pan variants format parse and speak explicitly',()=>{
  const descriptors=Object.fromEntries(['percent','unit-interval','bipolar-percent','bipolar-unit','pan'].map(kind=>[kind,createParameterVariant(kind)]));
  const formatCases=[
    ['percent',.54,'54%'],['unit-interval',.54,'.54'],
    ['bipolar-percent',.23,'−54%'],['bipolar-percent',.5,'0'],['bipolar-percent',.77,'+54%'],
    ['bipolar-unit',.23,'−.54'],['bipolar-unit',.5,'0'],['bipolar-unit',.77,'+.54'],
    ['pan',.23,'L 54%'],['pan',.5,'C'],['pan',.77,'R 54%'],
  ];
  for(const [kind,position,expected] of formatCases)assert.equal(descriptors[kind].format(position),expected,`${kind} at ${position}`);
  const parseCases=[
    ['percent','54%',.54],['unit-interval',' 0.54 ',.54],
    ['pan','L54',-.54],['pan','54% L',-.54],['pan','R 54%',.54],['pan','C',0],
  ];
  for(const [kind,text,expected] of parseCases)closeTo(descriptors[kind].parseEdit(text).nativeValue,expected);
  const accessibleCases=[['unit-interval',.54,'0.54'],['pan',.23,'54 percent left'],['pan',.5,'centre'],['pan',.77,'54 percent right']];
  for(const [kind,position,expected] of accessibleCases)assert.equal(descriptors[kind].accessibleText(position),expected,`${kind} accessible text`);
});

test('signed fractions map through the normalized range and retain their exact expression',()=>{
  const logarithmic=createParameterDescriptor({kind:'physical',min:20,max:20000,defaultValue:20,scale:'log',unit:'Hz'});
  const third=logarithmic.parseEdit('1/3');
  assert.equal(third.valid,true);assert.deepEqual(third.rational,{numerator:1,denominator:3});
  assert.ok(Math.abs(third.position-1/3)<1e-12);assert.ok(Math.abs(third.nativeValue-200)<1e-9);
  const bipolar=createParameterVariant('bipolar-unit');
  const positive=bipolar.parseEdit('1/3');const negative=bipolar.parseEdit('-1/3');
  closeTo(positive.nativeValue,1/3);closeTo(negative.nativeValue,-1/3);
  assert.ok(Math.abs(positive.position-.5-(.5-negative.position))<1e-12);
  assert.deepEqual(bipolar.parseEdit('2/6').rational,{numerator:2,denominator:6});
  assert.equal(bipolar.parseEdit('1/0').valid,false);assert.equal(bipolar.parseEdit('1/-3').valid,false);
});

test('retained fraction steps preserve the denominator and clamp symmetrically',()=>{
  const unit=createParameterVariant('unit-interval');
  let result=stepRational({numerator:1,denominator:3},1,unit);assert.deepEqual(result.rational,{numerator:2,denominator:3});assert.equal(result.position,2/3);
  result=stepRational(result.rational,1,unit);assert.deepEqual(result.rational,{numerator:3,denominator:3});
  result=stepRational(result.rational,1,unit);assert.deepEqual(result.rational,{numerator:3,denominator:3});
  const bipolar=createParameterVariant('bipolar-unit');
  result=stepRational({numerator:-1,denominator:6},1,bipolar);assert.deepEqual(result.rational,{numerator:0,denominator:6});assert.equal(result.position,.5);
  result=stepRational({numerator:2,denominator:6},1,bipolar);assert.deepEqual(result.rational,{numerator:3,denominator:6});assert.equal(result.nativeValue,.5);
  result=stepRational({numerator:1,denominator:6},2,bipolar);assert.deepEqual(result.rational,{numerator:3,denominator:6});
  assert.equal(fractionDragSteps(6,false),1);assert.equal(fractionDragSteps(6,true),0);assert.equal(fractionDragSteps(24,true),1);
});

test('physical pitch decibel stepped and enumerated descriptors share parsing and snapping',()=>{
  const cases=[
    ['frequency',{min:20,max:20000,defaultValue:1000},'2 kHz',2000],
    ['time',{min:.001,max:10,unit:'s'},'250 ms',.25],['time',{min:.001,max:10,unit:'s'},'2 s',2],
    ['pitch',{min:-24,max:24,step:1},'12.4 st',12],['cents',{min:-100,max:100,step:1},'−14 ct',-14],
    ['decibel',{min:-60,max:12,step:.1},'-6 dB',-6],['integer',{min:1,max:16},'4.7',5],
  ];
  for(const [kind,options,text,expected] of cases)assert.equal(createParameterVariant(kind,options).parseEdit(text).nativeValue,expected,`${kind} parses ${text}`);
  const frequency=createParameterVariant('frequency',{min:20,max:20000,defaultValue:1000});
  assert.equal(frequency.accessibleText(frequency.toPosition(2000)),'2000 hertz');
  const slope=createParameterVariant('enum',{values:[12,24,48],labels:['12 dB/oct','24 dB/oct','48 dB/oct']});
  assert.equal(slope.parseEdit('31').nativeValue,24);assert.equal(slope.parseEdit('48 dB/oct').nativeValue,48);assert.equal(slope.format(slope.toPosition(24)),'24 dB/oct');
});

test('tempo notes require explicit notation while bare fractions remain range proportions',()=>{
  const tempo=createParameterVariant('tempo-note',{min:.125,max:4,defaultValue:1});
  for(const [text,expected] of [['1/8 note',.5],['1/8D',.75],['1/8T',1/3]])closeTo(tempo.parseEdit(text).nativeValue,expected);
  const bare=tempo.parseEdit('1/8');assert.deepEqual(bare.rational,{numerator:1,denominator:8});assert.equal(bare.position,1/8);
});
