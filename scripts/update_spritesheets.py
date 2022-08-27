import pypdn
import imageio
import pathlib
import os
import re
from glob import glob
import numpy as np

# decorator
def _progress(fn):
    def _inner(source, *args, **kwargs):
        if isinstance(source, str):
            filen = source.replace('\\','/').split('/assets-dev/')[1]
            print("{:.<40}".format(filen), end='',flush=True)
        fn(source, *args, **kwargs)
            
        if isinstance(source, str):
            print("done")
    return _inner

def read_png_or_pdn(filen):
    extension = filen.split('.')[-1]
    if extension == "pdn":
        return pypdn.read(filen).flatten(asByte=True)
    elif extension == 'png':
        return imageio.imread(filen)
    else:
        raise ValueError("Filename must be a png or pdn file")

@_progress
def update_pdn_spritesheet(pdn_file, dest_file):
    image = pypdn.read(pdn_file)
    image.layers = list(filter(
        lambda s: s.name.lower() != 'background',
        image.layers
    ))
    image = image.flatten(asByte=True)
    
    imageio.imwrite(dest_file, image)
    
@_progress
def upscale_ui_element(source_file, dest_file, scale_factor=2, split_ct=None):
    src_image = read_png_or_pdn(source_file)
    
    if split_ct is None:
        dest_image = upscale_indiv(src_image, scale_factor)
        imageio.imwrite(dest_file, dest_image)
    else:
        split_size = src_image.shape[1] // split_ct
        
        for i in range(split_ct):
            dest_image = upscale_indiv(src_image[:,split_size*i:split_size*(i+1),:], scale_factor)
            imageio.imwrite(dest_file.format(i), dest_image)
        
    
def upscale_indiv(image, scale_factor):
    dest_shape = (
        scale_factor * image.shape[0],
        scale_factor * image.shape[1],
        image.shape[2]
    )
    dest_image = np.empty(dest_shape, dtype=image.dtype)
    
    for i in range(scale_factor):
        for j in range(scale_factor):
            dest_image[i::scale_factor,j::scale_factor] = image
    return dest_image
    

if __name__ == '__main__':
    # Figure out filepaths
    cargo_root = pathlib.Path(__file__).parent.parent.resolve()
    
    pdn_folder = os.path.join(cargo_root, 'assets-dev')
    assets = os.path.join(cargo_root, 'assets')

    # Go through the images
    items = [
        ('player.pdn', 'player/player.png'),
        ('player-staff.pdn', 'player/player-staff.png'),
        ('spiky-enemy.pdn', 'enemies/spiky-enemy.png'),
        ('gate.pdn', 'level/gate.png'),
    ]
    
    for source, dest in items:
        update_pdn_spritesheet(
            os.path.join(pdn_folder,source), 
            os.path.join(assets,dest)
        )
    
    # Update UI items
    # Can either be png or pdn
    # These need to be upscaled to work properly
    ui_source_dir = list(glob(os.path.join(pdn_folder, "ui", "*.png"))) + list(glob(os.path.join(pdn_folder, "ui", "*.pdn")))
    
    for source in ui_source_dir:
        source = source.replace('\\','/')
        
        expr = re.compile(r'/split-([0-9]+)-')
        match = expr.search(source)
        if match:
            end_filen = expr.split(source)[-1].replace('.pdn','.png').replace('.png','-{}.png')
            dest = os.path.join(assets, 'ui', end_filen)
            
            split_ct = int(match.group(1))
        else:
            end_filen = source.split('ui/')[1].replace('.pdn','.png')
            dest = os.path.join(assets, 'ui', end_filen)
            
            split_ct = None
            
        upscale_ui_element(source, dest, split_ct=split_ct)