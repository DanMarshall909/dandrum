import { readFileSync } from 'node:fs';

// Keep the unmodified notices with every JS bundle accompanying embedded fonts.
export const fontLicenseBanner = ['Barlow', 'BarlowSemiCondensed', 'JetBrainsMono']
  .map(family => `/*! ${family} fonts\n${readFileSync(new URL(`../../ui/design-system/fonts/${family}/OFL.txt`, import.meta.url), 'utf8')}*/`)
  .join('\n');
