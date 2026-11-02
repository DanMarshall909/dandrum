import {test,expect} from './fixtures.mjs';
test('source coverage observes the called JSX callback without calling the other paths',async({page})=>{
  await page.goto('/tests/coverage-fixtures/index.html');
  await page.getByRole('button',{name:'first',exact:true}).click();
  await expect(page.getByRole('button',{name:'hit 😀',exact:true})).toBeVisible();
});
