import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { app } from './lib/app.svelte';
import { startTheme } from './lib/themeBoot';

void startTheme();
app.init().catch((e) => console.error('init failed', e));

export default mount(App, { target: document.getElementById('app')! });
