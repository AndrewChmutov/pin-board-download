import selenium
import json


url = input("Enter the url of Pinterest board: ")

# Firefox driver
driver = './geckodriver'

# Get email and password from json
with open('info.json') as info_file:
    info = json.load(info_file)