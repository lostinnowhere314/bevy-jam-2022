import pypdn
import imageio
import pathlib
import os

# TODO script that goes through and manually updates things we're working on as .pdn files

def update_pdn_spritesheet(pdn_file, dest_file):
    
    
    image = pypdn.read(pdn_file)
    image.layers = list(filter(
        lambda s: s.name.lower() != 'background',
        image.layers
    ))
    image = image.flatten(asByte=True)
    
    imageio.imwrite(dest_file, image)

if __name__ == '__main__':
    # Figure out filepaths
    cargo_root = pathlib.Path(__file__).parent.parent.resolve()
    
    pdn_folder = os.path.join(cargo_root, 'assets-dev')
    assets = os.path.join(cargo_root, 'assets')

    # Go through the images
    items = [
        ('player.pdn', 'player/player.png')
    ]
    
    for source, dest in items:
        update_pdn_spritesheet(
            os.path.join(pdn_folder,source), 
            os.path.join(assets,dest)
        )
    
    # Update UI items