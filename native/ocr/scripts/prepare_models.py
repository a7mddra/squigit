from pathlib import Path
import shutil
import urllib.request

root = Path(__file__).resolve().parents[1] / "models"
for name in ("PP-OCRv5_mobile_det", "en_PP-OCRv5_mobile_rec"):
    target = root / name
    target.mkdir(parents=True, exist_ok=True)
    for filename in ("inference.json", "inference.pdiparams", "inference.yml"):
        destination = target / filename
        if destination.is_file():
            continue
        cached = Path.home() / ".paddlex/official_models" / name / filename
        if cached.is_file():
            shutil.copy2(cached, destination)
        else:
            url = f"https://huggingface.co/PaddlePaddle/{name}/resolve/main/{filename}"
            temporary = destination.with_suffix(".download")
            print(f"Downloading {name}/{filename}", flush=True)
            with urllib.request.urlopen(url, timeout=180) as response, temporary.open("wb") as output:
                shutil.copyfileobj(response, output)
            temporary.replace(destination)
