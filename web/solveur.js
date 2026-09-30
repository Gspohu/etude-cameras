// Runs the placement search off the page thread, a whole search freezes a tab for a minute

import init, { placer } from "./pkg/champ.js";

let demarre = null;

self.onmessage = async (ev) =>
{
    if (demarre === null)
    {
        demarre = init();
    }
    try
    {
        await demarre;
        self.postMessage({ ok: true, resultat: JSON.parse(placer(JSON.stringify(ev.data))) });
    }
    catch (e)
    {
        self.postMessage({ ok: false, erreur: String(e.message || e) });
    }
};
