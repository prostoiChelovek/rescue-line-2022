import numpy as np
import cv2 as cv
img = cv.imread(r"C:\Users\User\Downloads\cb1\grencvacva.png",1)
#You can access a pixel value by its row and column coordinates. For BGR image, it returns an array of Blue, Green, Red values. For grayscale image, just corresponding intensity is returned.
arr = np.asarray(img, dtype='uint8')#преобразует картинку в масив
red = img[:,:,2]
green=img[:,:,1]
blue0=img[:,:,0]

img[np.where(red>=230)]=0


red = img[:,:,2]=0
green=img[:,:,1]
blue0=img[:,:,0]=0


aq=img[:,:]
cv.imshow("window_name21",img)
cv.waitKey(0)
cv.destroyAllWindows()
print(aq)
