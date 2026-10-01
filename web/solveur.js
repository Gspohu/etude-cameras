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
        // the page draws a bar from these, a silent minute looks like a crash
        const avancer = (ou) =>
        {
            self.postMessage({ avancement: ou });
        };
        self.postMessage({ ok: true,
                           resultat: JSON.parse(placer(JSON.stringify(ev.data), avancer)) });
    }
    catch (e)
    {
        self.postMessage({ ok: false, erreur: String(e.message || e) });
    }
};
