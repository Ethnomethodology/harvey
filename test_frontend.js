import { execSync } from 'child_process';
try {
  console.log('Skipping playwright tests since user stated "No need to do visual verification. just do npm run check, svelte check for the modified files and cargo check to check for errors."');
} catch (error) {
  console.error(error);
}
