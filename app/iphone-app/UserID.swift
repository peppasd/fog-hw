//
//  UserID.swift
//  iphone-app
//
//  Created by Könnecke, Alyssa on 03.07.23.
//

import Foundation
import SwiftUI

class UserID: Identifiable, Codable{
    var id: UUID
    init(id: UUID) {
            self.id = id
        }
}


@MainActor class UserData: ObservableObject {
    @Published var id: [UserID]
        
        init() {
            let userID = UserID(id: UUID())
            id = [userID]
        }
 }
