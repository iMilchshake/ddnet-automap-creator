let scriptLoad = null;

function loadClassicScript() {
    if (scriptLoad === null) {
        scriptLoad = new Promise((resolve, reject) => {
            const script = document.createElement("script");
            script.src = "rpp/rpp.js";
            script.onload = () => resolve();
            script.onerror = () => reject(new Error("could not load rpp/rpp.js"));
            document.head.append(script);
        });
    }

    return scriptLoad;
}

export async function compile_rules(source, output_file) {
    await loadClassicScript();

    const log = [];
    const record = (line) => log.push(line);

    // rpp keeps state across a run, so a reused instance would report the
    // errors of an earlier compile.
    const rpp = await createRpp({ print: record, printErr: record });

    rpp.FS.chdir("/rpp");
    rpp.FS.writeFile("/rpp/input.r", source);

    const code = rpp.callMain(["-p", "input.r"]);
    if (code !== 0) {
        throw new Error(log.join("\n"));
    }

    try {
        return rpp.FS.readFile("/rpp/" + output_file, { encoding: "utf8" });
    } catch {
        throw new Error(log.join("\n") || `rpp wrote no ${output_file}`);
    }
}
