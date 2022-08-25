import numpy as np
import pypdn
import argparse
import imageio


def main(w, h, in_name, out_name):
    # Get a filename we can format numbers into
    out_split = out_name.split('.')
    if out_split[-1] == 'pdn':
        out_split = out_split[:-1]
    out_split[-1] = out_split[-1] + '_{}'
    out_fmt_name = '.'.join(out_split) + '.png'
    
    image = pypdn.read(in_name)
    
    for i,layer in enumerate(image.layers):
        data = layer.image
        intermediate = np.column_stack([data] * w)
        final = np.row_stack([intermediate] * h)
        
        imageio.imwrite(out_fmt_name.format(i), final)

def expand_player_spritesheet(in_name, out_name):
    # Get a filename we can format numbers into
    out_split = out_name.split('.')
    if out_split[-1] == 'pdn':
        out_split = out_split[:-1]
    out_split[-1] = out_split[-1] + '_{}'
    out_fmt_name = '.'.join(out_split) + '.png'
    
    image = pypdn.read(in_name)
    
    for i_layer,layer in enumerate(image.layers):
        data = layer.image
        out_layer = np.zeros((512,48*8,4), dtype=data.dtype)
        for i in range(8):
            out_layer[:,8+48*i:40+48*i,:] = data[:,i*32:(i+1)*32,:]
        
        imageio.imwrite(out_fmt_name.format(i_layer), out_layer)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Tiles the given pdn image')
    parser.add_argument('w', type=int)
    parser.add_argument('h', type=int)
    parser.add_argument('in_file')
    parser.add_argument('out_file', nargs='?', default=None)
    
    args = parser.parse_args()
    
    in_file = args.in_file
    out_file = args.out_file
    if out_file is None:
        out_file = in_file
        
    main(args.w, args.h, in_file, out_file)
    